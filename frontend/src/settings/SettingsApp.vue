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

    <p v-if="!unlocked" class="banner">
      保险库已锁定，请按 Ctrl+Alt+Space 唤出搜索窗解锁后再操作。
    </p>

    <main>
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
          <p class="muted">当前：<code>Ctrl + Alt + Space</code></p>
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

        <p v-if="notice" :class="['notice', noticeKind]">{{ notice }}</p>
      </section>

      <!-- 安全 -->
      <section v-else-if="tab === 'security'" class="pane">
        <div class="card">
          <h2>完整性检查</h2>
          <button :disabled="busy" @click="checkIntegrity">检查数据库</button>
          <dl v-if="integrity">
            <dt>SQLite</dt><dd>{{ integrity.sqlite_ok ? '正常' : '异常' }}</dd>
            <dt>数据库大小</dt><dd>{{ (integrity.db_size / 1024).toFixed(1) }} KB</dd>
            <dt>与锁定基线</dt><dd>{{ integrity.baseline_ok ? '一致' : '不一致（可能被外部修改）' }}</dd>
          </dl>
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
          <h2>SecurePass 1.0.0</h2>
          <p class="muted">本地密码管理器：AES-256-GCM + PBKDF2-HMAC-SHA256（600,000 次）。</p>
          <p class="muted">数据不会离开这台设备。</p>
        </div>
      </section>
    </main>

    <p v-if="error" class="error">{{ error }}</p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { request } from '../shared/ipc';

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

onMounted(async () => {
  autoLock.value = Number(localStorage.getItem('sp.autoLockMinutes') ?? '0') || 0;
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
