<template>
  <div class="panel" :class="{ tall: isTall }">
    <!-- 搜索 -->
    <template v-if="mode === 'search'">
      <div class="searchbar">
        <span class="glyph">🔒</span>
        <input
          ref="searchInput"
          v-model="query"
          class="search-input"
          placeholder="搜索站点、用户名、标签…"
          spellcheck="false"
          autocomplete="off"
          @keydown="onSearchKey"
          @compositionstart="composing = true"
          @compositionend="composing = false"
        />
        <kbd>Esc</kbd>
      </div>

      <ul class="results">
        <li
          v-for="(r, i) in results"
          :key="r.item.id"
          :class="{ active: i === cursor }"
          @mouseenter="cursor = i"
          @click="openDetail(r.item)"
        >
          <span class="avatar" :style="{ background: avatarColor(r.item.site_name) }">
            {{ firstChar(r.item.site_name) }}
          </span>
          <span class="name">
            <template v-for="(seg, j) in segmentsFor(r)" :key="j">
              <mark v-if="seg.hit">{{ seg.text }}</mark>
              <template v-else>{{ seg.text }}</template>
            </template>
          </span>
          <span class="user">{{ r.item.username }}</span>
          <span class="tags">{{ r.item.tags.slice(0, 2).join(' · ') }}</span>
        </li>
        <li v-if="!results.length" class="empty">
          {{ query ? '没有匹配的条目' : '库还是空的，按 Ctrl+N 新建' }}
        </li>
      </ul>

      <div class="hintbar">
        <span>↑↓ 选择</span><span>↵ 复制密码</span><span>Ctrl+↵ 用户名</span>
        <span>→ 详情</span><span>Ctrl+N 新建</span>
      </div>
    </template>

    <!-- 首次设置 -->
    <form v-else-if="mode === 'setup'" class="auth" @submit.prevent="doSetup">
      <div class="auth-icon">🔒</div>
      <h1>设置主密码</h1>
      <p class="muted">至少 12 位，包含大小写字母、数字和特殊字符</p>
      <p v-if="error" class="error">{{ error }}</p>
      <input v-model="password" type="password" placeholder="主密码" autocomplete="new-password" @keydown.esc.prevent="hide" />
      <input v-model="password2" type="password" placeholder="再输一次" autocomplete="new-password" @keydown.esc.prevent="hide" />
      <button type="submit" :disabled="busy">创建保险库</button>
    </form>

    <!-- 解锁 -->
    <form v-else-if="mode === 'unlock'" class="auth" @submit.prevent="doUnlock">
      <div class="auth-icon">🔒</div>
      <h1>解锁 SecurePass</h1>
      <p v-if="error" class="error">{{ error }}</p>
      <input
        ref="authInput"
        v-model="password"
        type="password"
        placeholder="主密码"
        autocomplete="current-password"
        @keydown.esc.prevent="hide"
      />
      <button type="submit" :disabled="busy">解锁</button>
    </form>

    <!-- 详情 -->
    <section v-else-if="mode === 'detail' && detail" class="detail">
      <header class="detail-head">
        <span class="avatar big" :style="{ background: avatarColor(detail.site_name) }">
          {{ firstChar(detail.site_name) }}
        </span>
        <div>
          <h2>{{ detail.site_name }}</h2>
          <p class="muted">{{ detail.username || '无用户名' }}</p>
        </div>
      </header>
      <p v-if="error" class="error">{{ error }}</p>
      <dl>
        <dt>密码</dt>
        <dd class="secret">
          <code>{{ revealed ?? '••••••••••' }}</code>
          <button class="mini" @click="copy('password')">复制</button>
        </dd>
        <dt>网址</dt>
        <dd>{{ detail.site_url || '—' }}</dd>
        <dt>标签</dt>
        <dd>{{ detail.tags.join(' / ') || '—' }}</dd>
        <dt>备注</dt>
        <dd class="notes">{{ detail.notes || '—' }}</dd>
      </dl>
      <div class="hintbar">
        <span>按住 Alt 看密码</span><span>E 编辑</span><span>Del 两次删除</span><span>Esc 返回</span>
      </div>
    </section>

    <!-- 新建 / 编辑 -->
    <form v-else-if="mode === 'edit'" class="edit" @submit.prevent="save">
      <h2>{{ editingId ? '编辑条目' : '新建条目' }}</h2>
      <p v-if="error" class="error">{{ error }}</p>
      <label>站点名<input v-model="form.site_name" required /></label>
      <label>用户名<input v-model="form.username" /></label>
      <label>
        密码
        <input
          v-model="form.password"
          type="password"
          :placeholder="editingId ? '留空则不修改' : '必填'"
          autocomplete="new-password"
        />
      </label>
      <label>网址<input v-model="form.site_url" placeholder="https://" /></label>
      <label>标签<input v-model="form.tags" placeholder="用逗号分隔" /></label>
      <label>备注<textarea v-model="form.notes" rows="2"></textarea></label>
      <div class="actions">
        <button type="submit" :disabled="busy">保存</button>
        <button type="button" class="ghost" @click="cancelEdit">取消</button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { api, listen, type EntryDetail, type Summary } from './ipc';
import { highlight, search as fuzzySearch } from './fuzzy';

type Mode = 'setup' | 'unlock' | 'search' | 'detail' | 'edit';

const mode = ref<Mode>('unlock');
const items = ref<Summary[]>([]);
const query = ref('');
const cursor = ref(0);
const password = ref('');
const password2 = ref('');
const revealed = ref<string | null>(null);
const detail = ref<EntryDetail | null>(null);
const busy = ref(false);
const error = ref('');
const composing = ref(false);
const editingId = ref<number | null>(null);
const confirmDelete = ref(false);

const searchInput = ref<HTMLInputElement>();
const authInput = ref<HTMLInputElement>();

const form = reactive({
  site_name: '',
  username: '',
  password: '',
  site_url: '',
  tags: '',
  notes: '',
});

const results = computed(() => fuzzySearch(items.value, query.value));
watch(results, (list) => {
  if (cursor.value >= list.length) cursor.value = 0;
});
const isTall = computed(() => mode.value === 'detail' || mode.value === 'edit');

function firstChar(name: string) {
  return (name || '?').trim().slice(0, 1).toUpperCase();
}

function avatarColor(name: string) {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return `hsl(${h} 55% 42%)`;
}

function segmentsFor(r: { item: Summary; ranges: [number, number][] }) {
  return highlight(r.item.site_name, r.ranges);
}

function resetSensitive() {
  revealed.value = null;
  detail.value = null;
  password.value = '';
  password2.value = '';
  form.password = '';
}

async function refresh() {
  items.value = await api.summaries();
}

async function goSearch() {
  mode.value = 'search';
  query.value = '';
  cursor.value = 0;
  await api.suppressHide(false);
  await api.resize(420);
  await nextTick();
  searchInput.value?.focus();
}

async function onShown() {
  error.value = '';
  if (mode.value === 'edit') return;
  try {
    if (!(await api.isSet())) {
      mode.value = 'setup';
      await api.suppressHide(true);
      await nextTick();
      return;
    }
    if (!(await api.status())) {
      mode.value = 'unlock';
      await api.suppressHide(true);
      await nextTick();
      authInput.value?.focus();
      return;
    }
    await refresh();
    await goSearch();
  } catch (e) {
    // 状态查询失败时按锁定处理，绝不展示缓存条目
    resetSensitive();
    items.value = [];
    mode.value = 'unlock';
    error.value = (e as Error).message;
    await api.suppressHide(true);
  }
}

/// 窗口重新可见/获得焦点时向后端确认一次，防止事件丢失后仍显示旧结果。
async function ensureLockState() {
  if (mode.value === 'edit' || mode.value === 'setup') return;
  let unlocked = false;
  try {
    unlocked = await api.status();
  } catch {
    unlocked = false;
  }
  if (unlocked) return;
  resetSensitive();
  items.value = [];
  mode.value = 'unlock';
  await api.suppressHide(true);
  await nextTick();
  authInput.value?.focus();
}

async function doSetup() {
  error.value = '';
  if (password.value !== password2.value) {
    error.value = '两次输入不一致';
    return;
  }
  busy.value = true;
  try {
    await api.setup(password.value);
    password.value = '';
    password2.value = '';
    await refresh();
    await goSearch();
  } catch (e) {
    error.value = (e as Error).message;
  } finally {
    busy.value = false;
  }
}

async function doUnlock() {
  error.value = '';
  busy.value = true;
  try {
    await api.unlock(password.value);
    password.value = '';
    await refresh();
    await goSearch();
  } catch (e) {
    error.value = (e as Error).message;
    password.value = '';
  } finally {
    busy.value = false;
  }
}

function move(delta: number) {
  const n = results.value.length;
  if (!n) return;
  cursor.value = (cursor.value + delta + n) % n;
}

async function copy(field: 'password' | 'username') {
  const entry = detail.value ?? results.value[cursor.value]?.item;
  if (!entry) return;
  try {
    await api.copy(entry.id, field);
    await hide();
  } catch (e) {
    error.value = (e as Error).message;
  }
}

async function hide() {
  resetSensitive();
  await api.suppressHide(false);
  await api.hide();
}

function onSearchKey(e: KeyboardEvent) {
  if (composing.value) return;
  const key = e.key;
  if (key === 'ArrowDown') {
    e.preventDefault();
    move(1);
  } else if (key === 'ArrowUp') {
    e.preventDefault();
    move(-1);
  } else if (key === 'Enter') {
    e.preventDefault();
    void copy(e.ctrlKey || e.metaKey ? 'username' : 'password');
  } else if (key === 'Tab' || key === 'ArrowRight') {
    e.preventDefault();
    const entry = results.value[cursor.value]?.item;
    if (entry) void openDetail(entry);
  } else if (key === 'Escape') {
    e.preventDefault();
    if (query.value) query.value = '';
    else void hide();
  } else if (key.toLowerCase() === 'n' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    void startNew();
  }
}

async function openDetail(item: Summary) {
  error.value = '';
  try {
    detail.value = await api.detail(item.id);
    mode.value = 'detail';
    revealed.value = null;
    confirmDelete.value = false;
    await api.suppressHide(false);
    await api.resize(560);
  } catch (e) {
    error.value = (e as Error).message;
  }
}

async function startNew() {
  editingId.value = null;
  Object.assign(form, { site_name: '', username: '', password: '', site_url: '', tags: '', notes: '' });
  mode.value = 'edit';
  error.value = '';
  await api.suppressHide(true);
  await api.resize(560);
}

async function startEdit() {
  if (!detail.value) return;
  editingId.value = detail.value.id;
  Object.assign(form, {
    site_name: detail.value.site_name,
    username: detail.value.username,
    password: '',
    site_url: detail.value.site_url,
    tags: detail.value.tags.join(', '),
    notes: detail.value.notes || '',
  });
  mode.value = 'edit';
  error.value = '';
  await api.suppressHide(true);
  await api.resize(560);
}

function cancelEdit() {
  if (editingId.value && detail.value) mode.value = 'detail';
  else void goSearch();
}

async function save() {
  error.value = '';
  if (!form.site_name.trim()) {
    error.value = '站点名不能为空';
    return;
  }
  if (!editingId.value && !form.password) {
    error.value = '新建条目必须填写密码';
    return;
  }
  busy.value = true;
  try {
    const tags = form.tags
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    if (editingId.value) {
      const payload: Record<string, unknown> = {
        id: editingId.value,
        site_name: form.site_name.trim(),
        site_url: form.site_url.trim(),
        username: form.username,
        notes: form.notes,
        tags,
      };
      if (form.password) payload.password = form.password;
      await api.update(payload);
      detail.value = await api.detail(editingId.value);
      mode.value = 'detail';
    } else {
      await api.add({
        site_name: form.site_name.trim(),
        site_url: form.site_url.trim(),
        username: form.username,
        password: form.password,
        notes: form.notes,
        tags,
      });
      await goSearch();
    }
    form.password = '';
    await api.suppressHide(false);
    await api.resize(560);
    await refresh();
  } catch (e) {
    error.value = (e as Error).message;
  } finally {
    busy.value = false;
  }
}

async function removeEntry() {
  if (!detail.value) return;
  if (!confirmDelete.value) {
    confirmDelete.value = true;
    error.value = '再按一次 Delete 确认删除';
    return;
  }
  try {
    await api.remove(detail.value.id);
    confirmDelete.value = false;
    await refresh();
    await goSearch();
  } catch (e) {
    error.value = (e as Error).message;
  }
}

async function showPassword() {
  if (mode.value !== 'detail' || !detail.value || revealed.value) return;
  try {
    revealed.value = await api.reveal(detail.value.id);
  } catch (e) {
    error.value = (e as Error).message;
  }
}

function onGlobalKey(e: KeyboardEvent) {
  if (mode.value !== 'detail') return;
  if (e.key === 'Alt') {
    e.preventDefault();
    void showPassword();
  } else if (e.key === 'Escape') {
    e.preventDefault();
    revealed.value = null;
    void goSearch();
  } else if (e.key === 'e' || e.key === 'E') {
    e.preventDefault();
    void startEdit();
  } else if (e.key === 'Delete') {
    e.preventDefault();
    void removeEntry();
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (e.key === 'Alt') revealed.value = null;
}

let unlisten: Array<() => void> = [];

onMounted(async () => {
  // 恢复上次在设置窗里选择的自动锁时长（0 = 不自动锁）
  const minutes = Number(localStorage.getItem('sp.autoLockMinutes') ?? '0') || 0;
  await api.setSessionTimeout(minutes * 60).catch(() => {});
  window.addEventListener('keydown', onGlobalKey);
  window.addEventListener('keyup', onKeyUp);
  window.addEventListener('focus', () => void ensureLockState());
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) void ensureLockState();
  });
  unlisten.push(await listen('launcher-shown', () => void onShown()));
  unlisten.push(
    await listen('launcher-hidden', () => {
      revealed.value = null;
      items.value = [];
      if (mode.value === 'detail') void goSearch();
    })
  );
  unlisten.push(
    await listen('vault-locked', () => {
      resetSensitive();
      items.value = [];
      mode.value = 'unlock';
      void api.suppressHide(true);
    })
  );
  await onShown();
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onGlobalKey);
  window.removeEventListener('keyup', onKeyUp);
  unlisten.forEach((fn) => fn());
});

watch(mode, async (m) => {
  if (m === 'search') {
    cursor.value = 0;
    await nextTick();
    searchInput.value?.focus();
  }
  if (m === 'unlock' || m === 'setup') {
    await nextTick();
    authInput.value?.focus();
  }
});
</script>

<style>
@import './style.css';
</style>
