<template>
  <div class="settings">
    <header>
      <h1>SecurePass 设置</h1>
      <span v-if="unlocked" class="pill ok">已解锁</span>
      <span v-else class="pill warn">已锁定</span>
    </header>

    <nav class="tabs">
      <button v-for="t in tabs" :key="t.id" :class="{ active: tab === t.id }" @click="tab = t.id">
        {{ t.label }}
      </button>
    </nav>

    <p v-if="error" class="banner bad">{{ error }}</p>
    <p v-else-if="notice" class="banner" :class="noticeKind">{{ notice }}</p>
    <p v-if="!unlocked" class="banner">
      保险库已锁定，请按 Ctrl+Alt+Space 唤出搜索窗解锁后再操作。
    </p>

    <main :inert="!unlocked">
      <!-- 通用 -->
      <section v-if="tab === 'general'" class="pane">
        <div class="card">
          <h2>会话</h2>
          <label class="row">
            自动锁定
            <select v-model.number="autoLock" @change="saveAutoLock">
              <option :value="0">不自动锁定</option>
              <option :value="5">5 分钟</option>
              <option :value="15">15 分钟</option>
              <option :value="30">30 分钟</option>
            </select>
          </label>
          <p class="muted">启动器只在手动锁定或退出时清空解密数据。</p>
          <button class="ghost" @click="lockNow">立即锁定</button>
        </div>

        <div class="card">
          <h2>全局热键</h2>
          <div class="hotkey-row">
            <span class="hotkey-label">唤醒搜索</span>
            <kbd class="hotkey">{{ wakeDisplay }}</kbd>
            <button v-if="recordingKind !== 'wake'" class="ghost" @click="startRecord('wake')">修改</button>
            <button v-else class="ghost" @click="cancelRecord">取消</button>
          </div>
          <div class="hotkey-row">
            <span class="hotkey-label">立即锁定</span>
            <kbd class="hotkey">{{ lockDisplay }}</kbd>
            <button v-if="recordingKind !== 'lock'" class="ghost" @click="startRecord('lock')">修改</button>
            <button v-else class="ghost" @click="cancelRecord">取消</button>
          </div>
          <template v-if="recordingKind">
            <p class="muted">
              {{ capturedDisplay ? `将设置为 ${capturedDisplay}` : '请按下新的组合键（至少带 Ctrl / Alt / Shift，F1~F24 可单独使用）' }}
            </p>
            <div v-if="capturedAccel" class="actions">
              <button :disabled="savingHotkey" @click="saveHotkey">保存</button>
            </div>
            <p class="muted">录制期间两个热键都会临时注销；按 Esc 取消。</p>
          </template>
        </div>

        <div class="card">
          <h2>修改主密码</h2>
          <input v-model="oldPw" type="password" placeholder="旧主密码" autocomplete="current-password" />
          <input v-model="newPw" type="password" placeholder="新主密码（至少 12 位）" autocomplete="new-password" />
          <input v-model="newPw2" type="password" placeholder="再输一次" autocomplete="new-password" />
          <button :disabled="busy" @click="changeMaster">修改</button>
        </div>
      </section>

      <!-- 导入导出 -->
      <section v-else-if="tab === 'transfer'" class="pane">
        <div class="card">
          <h2>导入密码</h2>
          <p class="muted">支持加密导出包（.spb）以及 JSON / CSV 明文。</p>
          <input ref="importFile" type="file" accept=".spb,.json,.csv" @change="onImportFile" />
          <input v-if="needImportPw" v-model="importPw" type="password" placeholder="导出包密码" />
          <button :disabled="busy || !importPath" @click="doImport">导入</button>
        </div>

        <div class="card">
          <h2>导出密码</h2>
          <p class="muted">导出为加密包，需要单独设置一个导出密码（至少 8 位）。</p>
          <input v-model="exportPw" type="password" placeholder="导出密码" autocomplete="new-password" />
          <button :disabled="busy" @click="doExport">导出全部</button>
        </div>

        <div class="card">
          <h2>备份恢复</h2>
          <button class="ghost" :disabled="busy" @click="doBackup">导出备份</button>
          <input ref="restoreFile" type="file" accept=".spbbackup,.json" @change="onRestoreFile" />
          <input v-model="restorePw" type="password" placeholder="备份对应的主密码" />
          <button :disabled="busy || !restorePath" @click="doRestore">从备份恢复</button>
        </div>

      </section>

      <!-- 安全 -->
      <section v-else-if="tab === 'security'" class="pane">
        <div class="card">
          <h2>完整性检查</h2>
          <button :disabled="busy" @click="checkIntegrity">检查数据库</button>
          <dl v-if="integrity">
            <dt>SQLite</dt><dd>{{ integrity.sqlite_ok ? '正常' : '异常' }}</dd>
            <dt>数据库大小</dt><dd>{{ (integrity.db_size / 1024).toFixed(1) }} KB</dd>
            <dt>与基线</dt><dd>{{ integrity.baseline_ok ? '一致' : '不一致' }}</dd>
          </dl>
          <template v-if="integrity && !integrity.baseline_ok">
            <p class="muted warn-text">
              与上次正常关闭时记录的基线不一致。常见原因：上次没走「退出」而是直接结束了进程、断电，
              或用外部工具改过数据库。确认是你自己的改动，点下面按钮重建基线即可。
            </p>
            <button class="ghost" :disabled="busy" @click="rebuildBaseline">重建基线</button>
          </template>
        </div>

        <div class="card">
          <h2>统计</h2>
          <p>共 <strong>{{ stats?.total_count ?? '—' }}</strong> 条密码</p>
          <button class="ghost" :disabled="busy" @click="loadStats">刷新</button>
        </div>

        <div class="card">
          <h2>即将过期（90 天）</h2>
          <ul class="list">
            <li v-for="e in expiring" :key="e.id">
              <span>{{ e.site_name }}</span>
              <span class="muted">{{ e.expired ? '已过期' : `${e.days_until_expiry} 天后` }}</span>
            </li>
            <li v-if="!expiring.length" class="muted">暂无</li>
          </ul>
        </div>
      </section>

      <!-- 审计 -->
      <section v-else-if="tab === 'audit'" class="pane">
        <div class="card">
          <div class="card-head">
            <h2>最近操作</h2>
            <button class="ghost" :disabled="busy" @click="loadAudit">刷新</button>
          </div>
          <div class="row">
            <span class="muted">自动保留最近 30 天、最多 1000 条</span>
            <span class="muted">
              手动清理
              <select v-model.number="auditDays">
                <option :value="7">7 天</option>
                <option :value="30">30 天</option>
                <option :value="90">90 天</option>
              </select>
              前
              <button class="ghost" :disabled="busy" @click="purgeAudit">执行</button>
            </span>
          </div>
          <p class="muted">解锁、复制、增删改都会记一条；超出上限时自动删最旧的。</p>
          <table>
            <thead><tr><th>时间</th><th>动作</th><th>结果</th></tr></thead>
            <tbody>
              <tr v-for="log in audit" :key="log.id">
                <td class="muted">{{ log.timestamp.slice(0, 19).replace('T', ' ') }}</td>
                <td>{{ log.action }}</td>
                <td :class="log.success ? 'ok' : 'bad'">{{ log.success ? '成功' : '失败' }}</td>
              </tr>
              <tr v-if="!audit.length"><td colspan="3" class="muted">暂无记录</td></tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- 关于 -->
      <section v-else class="pane">
        <div class="card">
          <h2>SecurePass</h2>
          <p class="muted">版本 1.0.0</p>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { listen, request } from '../shared/ipc';

interface Integrity { sqlite_ok: boolean; db_size: number; baseline_ok: boolean; db_checksum: string }
interface Stats { total_count: number }
interface Expiry { id: number; site_name: string; days_until_expiry: number; expired: boolean }
interface AuditLog { id: number; timestamp: string; action: string; success: boolean }
interface ImportResult { imported_count: number; skipped_count: number; updated_count: number; error_count: number }

const tabs = [
  { id: 'general', label: '通用' },
  { id: 'transfer', label: '导入导出' },
  { id: 'security', label: '安全' },
  { id: 'audit', label: '审计' },
  { id: 'about', label: '关于' },
];
const tab = ref('general');
const unlocked = ref(false);
const busy = ref(false);
const error = ref('');
const notice = ref('');
const noticeKind = ref<'ok' | 'bad'>('ok');

const autoLock = ref(0);
const wakeDisplay = ref('Ctrl + Alt + Space');
const lockDisplay = ref('Ctrl + Alt + L');
const recordingKind = ref<'wake' | 'lock' | null>(null);
const capturedAccel = ref('');
const capturedDisplay = ref('');
const savingHotkey = ref(false);

const KEY_TOKENS: Record<string, string> = {
  Space: 'Space', Enter: 'Enter', Tab: 'Tab', Backquote: 'Backquote',
  Minus: 'Minus', Equal: 'Equal', BracketLeft: 'BracketLeft', BracketRight: 'BracketRight',
  Semicolon: 'Semicolon', Quote: 'Quote', Comma: 'Comma', Period: 'Period',
  Slash: 'Slash', Backslash: 'Backslash',
  ArrowUp: 'ArrowUp', ArrowDown: 'ArrowDown', ArrowLeft: 'ArrowLeft', ArrowRight: 'ArrowRight',
  Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', Insert: 'Insert', Delete: 'Delete',
};

function codeToken(code: string): string {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return KEY_TOKENS[code] ?? '';
}

function prettyToken(token: string): string {
  if (/^[A-Z]$/.test(token)) return token;
  if (/^[0-9]$/.test(token)) return token;
  return token;
}

interface HotkeyInfo {
  wake: string;
  wake_display: string;
  lock: string;
  lock_display: string;
}

function applyHotkeyInfo(info: HotkeyInfo) {
  wakeDisplay.value = info.wake_display;
  lockDisplay.value = info.lock_display;
}

async function loadHotkey() {
  const info = await guard(() => request<HotkeyInfo>('get_hotkey'));
  if (info) applyHotkeyInfo(info);
}

function startRecord(kind: 'wake' | 'lock') {
  capturedAccel.value = '';
  capturedDisplay.value = '';
  error.value = '';
  recordingKind.value = kind;
  void request('begin_hotkey_capture');
  window.addEventListener('keydown', onRecordKey, true);
  window.addEventListener('blur', onRecordBlur);
}

function stopRecord(restore: boolean) {
  window.removeEventListener('keydown', onRecordKey, true);
  window.removeEventListener('blur', onRecordBlur);
  recordingKind.value = null;
  if (restore) void request('end_hotkey_capture');
}

function onRecordBlur() {
  if (recordingKind.value) stopRecord(true);
}

function onRecordKey(e: KeyboardEvent) {
  if (!recordingKind.value) return;
  e.preventDefault();
  e.stopPropagation();
  const mods: string[] = [];
  if (e.ctrlKey) mods.push('Ctrl');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey) mods.push('Shift');
  if (e.metaKey) mods.push('Super');
  if (e.key === 'Escape' && mods.length === 0) {
    stopRecord(true);
    return;
  }
  const token = codeToken(e.code);
  if (!token) return; // 只按了修饰键
  const isFKey = /^F([1-9]|1[0-9]|2[0-4])$/.test(token);
  if (!mods.length && !isFKey) {
    error.value = '至少带一个修饰键（F1~F24 除外）';
    return;
  }
  capturedAccel.value = [...mods, token].join('+');
  capturedDisplay.value = [...mods.map((m) => (m === 'Super' ? 'Win' : m)), prettyToken(token)].join(' + ');
  error.value = '';
}

async function saveHotkey() {
  const kind = recordingKind.value;
  if (!kind) return;
  savingHotkey.value = true;
  try {
    const info = await request<HotkeyInfo>('set_hotkey', {
      kind,
      accelerator: capturedAccel.value,
    });
    applyHotkeyInfo(info);
    stopRecord(false);
    setNotice(kind === 'lock' ? '锁定热键已更新' : '唤醒热键已更新');
  } catch (e) {
    error.value = (e as Error).message;
    stopRecord(true);
  } finally {
    savingHotkey.value = false;
  }
}

function cancelRecord() {
  stopRecord(true);
}

const oldPw = ref('');
const newPw = ref('');
const newPw2 = ref('');

const importFile = ref<HTMLInputElement>();
const importPath = ref('');
const importPw = ref('');
const needImportPw = ref(false);
const exportPw = ref('');

const restoreFile = ref<HTMLInputElement>();
const restorePath = ref('');
const restorePw = ref('');

const integrity = ref<Integrity | null>(null);
const stats = ref<Stats | null>(null);
const expiring = ref<Expiry[]>([]);
const audit = ref<AuditLog[]>([]);
const auditDays = ref(30);

function setNotice(text: string, kind: 'ok' | 'bad' = 'ok') {
  notice.value = text;
  noticeKind.value = kind;
}

async function guard<T>(fn: () => Promise<T>): Promise<T | undefined> {
  error.value = '';
  busy.value = true;
  try {
    return await fn();
  } catch (e) {
    error.value = (e as Error).message;
    return undefined;
  } finally {
    busy.value = false;
  }
}

function readFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ''));
    reader.onerror = () => reject(reader.error ?? new Error('读取文件失败'));
    reader.readAsText(file);
  });
}

async function saveAutoLock() {
  localStorage.setItem('sp.autoLockMinutes', String(autoLock.value));
  const ok = await guard(() => request('set_session_timeout', { seconds: autoLock.value * 60 }));
  if (ok !== undefined) setNotice('自动锁定设置已更新');
}

async function lockNow() {
  await guard(async () => {
    await request('lock');
    unlocked.value = false;
  });
}

async function changeMaster() {
  if (newPw.value !== newPw2.value) {
    error.value = '两次输入的新密码不一致';
    return;
  }
  const ok = await guard(() => request('change_master_password', { oldPassword: oldPw.value, newPassword: newPw.value }));
  if (ok !== undefined) {
    oldPw.value = newPw.value = newPw2.value = '';
    setNotice('主密码已修改');
  }
}

function onImportFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  importPath.value = file ? file.name : '';
  needImportPw.value = !!file && file.name.toLowerCase().endsWith('.spb');
}

async function doImport() {
  const file = importFile.value?.files?.[0];
  if (!file) return;
  const done = await guard(async () => {
    const data = await readFile(file);
    const res = file.name.toLowerCase().endsWith('.spb')
      ? await request<ImportResult>('import_encrypted', { data, exportPassword: importPw.value })
      : await request<ImportResult>('import_passwords', { data });
    return res;
  });
  if (done) {
    setNotice(`导入 ${done.imported_count} 条，跳过 ${done.skipped_count} 条重复`);
    importPw.value = '';
    if (importFile.value) importFile.value.value = '';
    importPath.value = '';
  }
}

async function doExport() {
  const saved = await guard(async () => {
    const content = await request<string>('export_encrypted', { exportPassword: exportPw.value, ids: null });
    return request<string>('save_export_file', {
      filename: `securepass-export-${new Date().toISOString().slice(0, 10)}.spb`,
      content,
    });
  });
  if (saved) {
    exportPw.value = '';
    setNotice(`已导出到 ${saved}`);
  }
}

async function doBackup() {
  const saved = await guard(async () => {
    const content = await request<string>('export_backup');
    return request<string>('save_export_file', {
      filename: `securepass-backup-${new Date().toISOString().slice(0, 10)}.spbbackup`,
      content,
    });
  });
  if (saved) setNotice(`备份已保存到 ${saved}`);
}

function onRestoreFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  restorePath.value = file ? file.name : '';
}

async function doRestore() {
  const file = restoreFile.value?.files?.[0];
  if (!file) return;
  const res = await guard(async () => {
    const data = await readFile(file);
    return request<ImportResult>('restore_backup', { data, masterPassword: restorePw.value });
  });
  if (res) {
    setNotice(`恢复 ${res.imported_count} 条，跳过 ${res.skipped_count} 条重复`);
    restorePw.value = '';
    if (restoreFile.value) restoreFile.value.value = '';
    restorePath.value = '';
  }
}

async function checkIntegrity() {
  const res = await guard(() => request<Integrity>('check_integrity'));
  if (res) integrity.value = res;
}

async function rebuildBaseline() {
  const ok = await guard(async () => {
    await request('rebuild_baseline');
    return true;
  });
  if (ok) {
    setNotice('完整性基线已重建');
    await checkIntegrity();
  }
}

async function purgeAudit() {
  const removed = await guard(() =>
    request<number>('purge_old_audit_logs', { days: auditDays.value })
  );
  if (removed !== undefined) {
    setNotice(`已清理 ${removed} 条旧日志`);
    await loadAudit();
  }
}

async function loadStats() {
  const [s, e] = await guard(async () => {
    const s = await request<Stats>('get_password_stats');
    const e = await request<Expiry[]>('get_expiring_passwords', { withinDays: 90 });
    return [s, e] as const;
  }) ?? [];
  if (s) stats.value = s;
  if (e) expiring.value = e;
}

async function loadAudit() {
  const res = await guard(() => request<AuditLog[]>('get_audit_logs'));
  if (res) audit.value = res;
}

let unlistenLock: (() => void) | null = null;

async function refreshLockState() {
  try {
    const s = await request<boolean>('vault_status');
    if (s && !unlocked.value) {
      unlocked.value = true;
      setNotice('保险库已解锁');
      await loadStats();
      await loadAudit();
    } else if (!s) {
      applyLocked();
    }
  } catch {
    applyLocked();
  }
}

function applyLocked() {
  if (!unlocked.value) return;
  unlocked.value = false;
  integrity.value = null;
  stats.value = null;
  expiring.value = [];
  audit.value = [];
  stopRecord(true);
  setNotice('保险库已锁定，操作已禁用', 'bad');
}

onBeforeUnmount(() => {
  if (recordingKind.value) void request('end_hotkey_capture');
  unlistenLock?.();
  window.removeEventListener('focus', refreshLockState);
});

onMounted(async () => {
  autoLock.value = Number(localStorage.getItem('sp.autoLockMinutes') ?? '0') || 0;
  await loadHotkey();
  window.addEventListener('focus', refreshLockState);
  unlistenLock = await listen('vault-locked', applyLocked);
  const s = await guard(() => request<boolean>('vault_status'));
  unlocked.value = !!s;
  if (unlocked.value) {
    await loadStats();
    await loadAudit();
  }
});
</script>

<style>
@import './style.css';
</style>
