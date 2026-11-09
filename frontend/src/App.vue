<template>
  <div class="app">
    <el-config-provider :locale="zhCn">
      <div class="app-container">
        <!-- 顶部导航栏 -->
        <el-header class="header">
          <div class="header-content">
            <div class="logo">
              <h1>🔒 SecurePass</h1>
              <span class="subtitle">安全密码管理器</span>
            </div>
            <div class="header-actions">
              <el-button 
                type="primary" 
                @click="openExportDialog"
                :icon="Download"
              >
                导出密码
              </el-button>
              <el-button 
                type="info" 
                @click="showImportDialog = true"
                :icon="Upload"
              >
                导入密码
              </el-button>
              <el-button 
                type="success" 
                @click="showBackupDialog = true"
                :icon="DocumentCopy"
              >
                备份恢复
              </el-button>
              <el-button 
                type="primary" 
                plain
                @click="openSecurityOverview"
                :icon="Lock"
              >
                安全概览
              </el-button>
              <el-button 
                type="warning" 
                @click="showSettingsDialog = true"
                :icon="Setting"
              >
                设置
              </el-button>
              <el-button
                v-if="passwordsLoaded"
                type="danger"
                plain
                :icon="Lock"
                @click="lockVault('已手动锁定保险库')"
              >
                锁定
              </el-button>
            </div>
          </div>
        </el-header>

        <!-- 主要内容区域 -->
        <el-main class="main">
          <div class="content-wrapper">
            <!-- 欢迎信息 -->
            <div v-if="!passwordsLoaded" class="welcome-section">
              <el-card class="welcome-card">
                <div class="welcome-content">
                  <div class="welcome-icon">🔒</div>
                  <div class="welcome-text">
                    <h2>欢迎使用 SecurePass</h2>
                    <p>您的安全密码管理助手</p>
                  </div>
                  <div class="welcome-actions">
                    <el-button 
                      type="primary" 
                      size="large"
                      @click="startUsing"
                    >
                      开始使用
                    </el-button>
                    <el-button 
                      size="large"
                      @click="showTutorial"
                    >
                      使用教程
                    </el-button>
                  </div>
                </div>
              </el-card>
            </div>

            <!-- 密码管理器 -->
            <PasswordManager v-if="passwordsLoaded" :key="vaultVersion" />
          </div>
        </el-main>

        <!-- 底部信息 -->
        <el-footer class="footer">
          <div class="footer-content">
            <span>© 2024 SecurePass - 安全密码管理器</span>
            <span class="footer-divider">|</span>
            <span>版本 1.0.0</span>
            <span class="footer-divider">|</span>
            <span>开源项目</span>
          </div>
        </el-footer>
      </div>
    </el-config-provider>

    <!-- 安全概览对话框 -->
    <el-dialog
      v-model="showSecurityDialog"
      title="安全概览"
      width="640px"
    >
      <div v-loading="securityLoading" class="security-overview">
        <el-descriptions :column="2" border title="密码策略">
          <el-descriptions-item label="最长使用天数">
            {{ passwordPolicy ? passwordPolicy.max_age_days : '-' }}
          </el-descriptions-item>
          <el-descriptions-item label="历史密码保留">
            {{ passwordPolicy ? passwordPolicy.history_limit : '-' }} 条
          </el-descriptions-item>
        </el-descriptions>

        <div class="integrity-row">
          <el-button size="small" @click="runIntegrityCheck" :loading="integrityLoading">运行完整性检查</el-button>
          <span v-if="integrityReport" style="margin-left: 12px;">
            <el-tag :type="integrityReport.sqlite_ok ? 'success' : 'danger'" size="small">
              SQLite {{ integrityReport.sqlite_ok ? '正常' : '异常' }}
            </el-tag>
            <el-tag :type="integrityReport.baseline_ok ? 'success' : 'warning'" size="small" style="margin-left: 8px;">
              基线 {{ integrityReport.baseline_ok ? '一致' : '不一致' }}
            </el-tag>
            <span style="margin-left: 8px;">
              DB SHA-256：{{ integrityReport.db_checksum.slice(0, 16) }}…（{{ integrityReport.db_size }} 字节）
            </span>
          </span>
        </div>

        <h4 class="security-title">即将 / 已经过期（{{ expiringPasswords.length }}）</h4>
        <el-table :data="expiringPasswords" size="small" max-height="200" empty-text="没有即将过期的密码">
          <el-table-column prop="site_name" label="站点" />
          <el-table-column label="状态" width="140">
            <template #default="scope">
              <el-tag :type="scope.row.expired ? 'danger' : 'warning'" size="small">
                {{ scope.row.expired ? '已过期' : '剩 ' + scope.row.days_until_expiry + ' 天' }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>

        <h4 class="security-title">最近审计日志（{{ auditLogs.length }}）</h4>
        <el-table :data="auditLogs" size="small" max-height="220" empty-text="暂无审计记录">
          <el-table-column prop="timestamp" label="时间" width="210" />
          <el-table-column prop="action" label="操作" width="170" />
          <el-table-column label="结果" width="90">
            <template #default="scope">
              <el-tag :type="scope.row.success ? 'success' : 'danger'" size="small">
                {{ scope.row.success ? '成功' : '失败' }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
      </div>
      <template #footer>
        <el-button @click="showSecurityDialog = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 导出对话框 -->
    <el-dialog
      v-model="showExportDialog"
      title="导出密码"
      width="560px"
    >
      <el-alert
        type="info"
        :closable="false"
        show-icon
        title="导出为加密文件（AES-256-GCM）"
        description="需设置一个「导出密码」。文件只有用该密码才能解密，与保险库主密码无关，可安全用于跨设备 / 外部分发。"
      />

      <div class="export-select">
        <div class="export-select-bar">
          <span>选择要导出的条目（已选 {{ exportSelectedIds.length }} / {{ exportCandidates.length }}）</span>
          <span class="export-select-actions">
            <el-button size="small" @click="selectAllExports">全选</el-button>
            <el-button size="small" @click="clearExportSelection">清空</el-button>
          </span>
        </div>
        <el-table
          ref="exportTableRef"
          :data="exportCandidates"
          height="200"
          size="small"
          border
          @selection-change="onExportSelectionChange"
        >
          <el-table-column type="selection" width="46" />
          <el-table-column prop="site_name" label="站点" min-width="130" show-overflow-tooltip />
          <el-table-column prop="username" label="用户名" min-width="130" show-overflow-tooltip />
          <el-table-column prop="category" label="分类" width="80" />
        </el-table>
      </div>

      <el-form label-width="90px" style="margin-top: 16px;">
        <el-form-item label="导出密码">
          <el-input
            ref="exportPasswordInputRef"
            v-model="exportPassword"
            type="password"
            show-password
            placeholder="设置导出密码（导入时需输入相同密码）"
          />
        </el-form-item>
        <el-form-item label="确认密码">
          <el-input
            v-model="exportPasswordConfirm"
            type="password"
            show-password
            placeholder="再次输入导出密码"
          />
        </el-form-item>
      </el-form>
      <div class="export-info">
        <p>导出文件：securepass-export-*.spb（加密包，非明文）</p>
        <p>请牢记导出密码，遗忘后将无法解密该备份。</p>
      </div>
      <template #footer>
        <el-button @click="showExportDialog = false">取消</el-button>
        <el-button type="primary" @click="handleEncryptedExport" :loading="exporting">
          加密导出
        </el-button>
      </template>
    </el-dialog>

    <!-- 导入对话框 -->
    <el-dialog
      v-model="showImportDialog"
      title="导入密码"
      width="400px"
    >
      <el-alert
        type="info"
        :closable="false"
        show-icon
        title="导入加密备份（.spb）"
        description="选择由本程序加密导出的 .spb 文件，导入时会要求输入当时的「导出密码」进行解密，随后按本机主密码重新加密入库。"
      />
      <div class="import-options" style="margin-top: 16px;">
        <el-form :model="importForm" label-width="80px">
          <el-form-item label="备份文件">
            <div class="spb-picker">
              <el-button type="primary" @click="spbFileInput?.click()">选择 .spb 文件</el-button>
              <span v-if="importForm.fileName" class="spb-name">{{ importForm.fileName }}</span>
              <span v-else class="spb-hint">仅支持本程序加密导出的 .spb 文件</span>
            </div>
            <input
              ref="spbFileInput"
              type="file"
              accept=".spb"
              style="display: none"
              @change="handleSpbFileChange"
            />
          </el-form-item>
        </el-form>
      </div>
      <template #footer>
        <el-button @click="showImportDialog = false">取消</el-button>
        <el-button type="primary" @click="handleEncryptedImport" :loading="importing">
          导入
        </el-button>
      </template>
    </el-dialog>

    <!-- 备份恢复对话框 -->
    <el-dialog
      v-model="showBackupDialog"
      title="备份恢复"
      width="500px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-card class="backup-card">
            <template #header>
              <div class="card-header">
                <span>创建备份</span>
                <el-button type="primary" size="small" @click="createBackup">
                  创建备份
                </el-button>
              </div>
            </template>
            <div class="backup-info">
              <p>备份包含：</p>
              <ul>
                <li>所有密码条目</li>
                <li>配置信息</li>
                <li>系统设置</li>
              </ul>
              <p>备份文件将保存到本地</p>
            </div>
          </el-card>
        </el-col>
        <el-col :span="12">
          <el-card class="backup-card">
            <template #header>
              <div class="card-header">
                <span>恢复备份</span>
                <el-button type="success" size="small" @click="showBackupFileSelect">
                  选择备份
                </el-button>
              </div>
            </template>
            <div class="backup-info">
              <p>从备份文件恢复：</p>
              <ul>
                <li>密码条目</li>
                <li>配置信息</li>
                <li>系统设置</li>
              </ul>
              <p>⚠️ 恢复将覆盖现有数据</p>
            </div>
          </el-card>
          <input
            ref="backupFileInput"
            type="file"
            accept=".json,application/json"
            style="display: none"
            @change="handleBackupFile"
          />
        </el-col>
      </el-row>
    </el-dialog>

    <!-- 设置对话框 -->
    <el-dialog
      v-model="showSettingsDialog"
      title="设置"
      width="600px"
    >
      <el-form :model="settingsForm" label-width="120px">
        <el-form-item label="主密码">
          <div class="change-master-password">
            <el-input
              v-model="oldMasterPassword"
              type="password"
              show-password
              placeholder="当前主密码"
            />
            <el-input
              v-model="newMasterPassword"
              type="password"
              show-password
              placeholder="新主密码：至少 12 位，含大小写字母、数字与符号"
            />
            <el-input
              v-model="confirmMasterPassword"
              type="password"
              show-password
              placeholder="再次输入新主密码"
            />
            <div v-if="newMasterPassword" class="password-strength">
              <el-progress
                :percentage="newMasterPasswordStrength"
                :color="newMasterPasswordStrengthColor"
                :show-text="false"
              />
              <span :class="['strength-text', newMasterPasswordStrengthClass]">
                强度：{{ newMasterPasswordStrengthText }}
              </span>
            </div>
            <el-button
              type="primary"
              plain
              :loading="changingPassword"
              @click="changeMasterPassword"
            >
              修改主密码
            </el-button>
          </div>
        </el-form-item>
        
        <el-form-item label="自动锁定">
          <el-switch
            v-model="settingsForm.autoLock"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
        
        <el-form-item label="锁定时间">
          <el-slider
            v-model="settingsForm.lockSeconds"
            :min="10"
            :max="300"
            :step="10"
            :format-tooltip="(value) => `${value} 秒`"
          />
          <span class="settings-hint">默认 30 秒，最长 5 分钟；无操作即锁定</span>
        </el-form-item>
        
        <el-form-item label="自动备份">
          <el-switch
            v-model="settingsForm.autoBackup"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
        
        <el-form-item label="备份频率">
          <el-select v-model="settingsForm.backupFrequency" placeholder="选择备份频率">
            <el-option label="每天" value="daily" />
            <el-option label="每周" value="weekly" />
            <el-option label="每月" value="monthly" />
          </el-select>
        </el-form-item>
        
        <el-form-item label="通知设置">
          <el-switch
            v-model="settingsForm.notifications"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="showSettingsDialog = false">取消</el-button>
        <el-button type="primary" @click="saveSettings" :loading="savingSettings">
          保存设置
        </el-button>
      </template>
    </el-dialog>

    <!-- 主密码设置 / 解锁对话框 -->
    <el-dialog
      v-model="showAuth"
      :title="authMode === 'setup' ? '设置主密码' : '解锁保险库'"
      width="420px"
      :close-on-click-modal="false"
      :show-close="false"
    >
      <el-form label-width="90px" @submit.prevent>
        <el-form-item :label="authMode === 'setup' ? '新主密码' : '主密码'">
          <el-input
            v-model="masterPassword"
            type="password"
            show-password
            placeholder="至少12位，含大小写字母、数字与符号"
            @keyup.enter="submitAuth"
          />
          <div v-if="authMode === 'setup' && masterPassword" class="password-strength">
            <el-progress
              :percentage="masterPasswordStrength"
              :color="masterPasswordStrengthColor"
              :show-text="false"
            />
            <span :class="['strength-text', masterPasswordStrengthClass]">
              强度：{{ masterPasswordStrengthText }}
            </span>
            <ul class="strength-rules">
              <li
                v-for="rule in masterPasswordRules"
                :key="rule.label"
                :class="rule.ok ? 'ok' : 'bad'"
              >
                {{ rule.ok ? '✓' : '✗' }} {{ rule.label }}
              </li>
            </ul>
          </div>
        </el-form-item>
        <el-alert
          v-if="authError"
          :title="authError"
          type="error"
          :closable="false"
          show-icon
        />
      </el-form>
      <template #footer>
        <el-button @click="showAuth = false">稍后</el-button>
        <el-button type="primary" @click="submitAuth" :loading="authLoading">
          {{ authMode === 'setup' ? '设置并进入' : '解锁' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import zhCn from 'element-plus/dist/locale/zh-cn.mjs';
import { Download, Upload, DocumentCopy, Setting, Lock } from '@element-plus/icons-vue';
import PasswordManager from './components/PasswordManager.vue';
import { authApi, importExportApi, notificationApi, passwordApi, type PasswordEntry, type PasswordPolicy, type PasswordExpiryInfo, type AuditLogEntry, type IntegrityReport } from './api';

// 响应式数据
const passwordsLoaded = ref(false);
// 变更后自增，用作 PasswordManager 的 key，强制重新拉取列表
const vaultVersion = ref(0);

// 主密码设置 / 解锁弹窗状态
const showAuth = ref(false);
const authMode = ref<'setup' | 'unlock'>('unlock');
const masterPassword = ref('');
const authLoading = ref(false);
const authError = ref('');
const showExportDialog = ref(false);
const showImportDialog = ref(false);
const showBackupDialog = ref(false);
const showSettingsDialog = ref(false);
const exporting = ref(false);
const importing = ref(false);
const savingSettings = ref(false);

// 安全概览
const showSecurityDialog = ref(false);
const securityLoading = ref(false);
const passwordPolicy = ref<PasswordPolicy | null>(null);
const expiringPasswords = ref<PasswordExpiryInfo[]>([]);
const auditLogs = ref<AuditLogEntry[]>([]);
const integrityReport = ref<IntegrityReport | null>(null);
const integrityLoading = ref(false);

// 导出设置
const exportCandidates = ref<PasswordEntry[]>([]);
const exportSelectedIds = ref<number[]>([]);
const exportTableRef = ref();
const exportPasswordInputRef = ref();
const exportPassword = ref('');
const exportPasswordConfirm = ref('');

// 导入设置
const importForm = ref({
  file: null as File | null,
  fileName: '',
});
const spbFileInput = ref<HTMLInputElement | null>(null);

// 备份文件引用
const backupFileInput = ref();

// 修改主密码表单
const oldMasterPassword = ref('');
const newMasterPassword = ref('');
const confirmMasterPassword = ref('');
const changingPassword = ref(false);

// 设置表单
const settingsForm = ref({
  autoLock: true,
  lockSeconds: 30,
  autoBackup: true,
  backupFrequency: 'weekly',
  notifications: true,
});

// 计算属性
// 主密码规则清单：设置、修改主密码共用（与后端 MasterPasswordManager 一致）
const passwordRulesFor = (p: string) => [
  { label: '至少 12 个字符', ok: p.length >= 12 },
  { label: '包含大写字母 (A-Z)', ok: /[A-Z]/.test(p) },
  { label: '包含小写字母 (a-z)', ok: /[a-z]/.test(p) },
  { label: '包含数字 (0-9)', ok: /[0-9]/.test(p) },
  { label: '包含特殊符号 (!@#$…)', ok: /[^a-zA-Z0-9]/.test(p) },
];

const masterPasswordRules = computed(() => passwordRulesFor(masterPassword.value));
const newMasterPasswordRules = computed(() => passwordRulesFor(newMasterPassword.value));

const strengthOf = (rules: { ok: boolean }[]) =>
  Math.round((rules.filter((r) => r.ok).length / rules.length) * 100);

const strengthColor = (strength: number) => {
  if (strength < 40) return '#f56c6c';
  if (strength < 100) return '#e6a23c';
  return '#67c23a';
};

const strengthText = (strength: number) => {
  if (strength < 40) return '弱';
  if (strength < 100) return '中';
  return '强';
};

const strengthClass = (strength: number) => {
  if (strength < 40) return 'weak';
  if (strength < 100) return 'medium';
  return 'strong';
};

const newMasterPasswordStrength = computed(() => strengthOf(newMasterPasswordRules.value));
const newMasterPasswordStrengthColor = computed(() => strengthColor(newMasterPasswordStrength.value));
const newMasterPasswordStrengthText = computed(() => strengthText(newMasterPasswordStrength.value));
const newMasterPasswordStrengthClass = computed(() => strengthClass(newMasterPasswordStrength.value));

const masterPasswordStrength = computed(() => strengthOf(masterPasswordRules.value));
const masterPasswordStrengthColor = computed(() => strengthColor(masterPasswordStrength.value));
const masterPasswordStrengthText = computed(() => strengthText(masterPasswordStrength.value));
const masterPasswordStrengthClass = computed(() => strengthClass(masterPasswordStrength.value));

// 方法
const startUsing = () => {
  showAuth.value = true;
  notificationApi.showNotification('欢迎使用 SecurePass！', 'success');
};

const showTutorial = () => {
  notificationApi.showNotification('使用教程功能开发中...', 'info');
};

// ===================== 自动锁定 =====================
const SETTINGS_STORAGE_KEY = 'securepass.settings';
const IDLE_CHECK_INTERVAL_MS = 1000;
const ACTIVITY_EVENTS = ['mousemove', 'mousedown', 'keydown', 'touchstart', 'scroll'];
let idleTimer: number | undefined;
let lastActivityAt = Date.now();

const loadSettings = () => {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) return;
    const saved = JSON.parse(raw) as { autoLock?: boolean; lockSeconds?: number };
    if (typeof saved.autoLock === 'boolean') settingsForm.value.autoLock = saved.autoLock;
    if (typeof saved.lockSeconds === 'number') {
      settingsForm.value.lockSeconds = Math.min(300, Math.max(10, Math.round(saved.lockSeconds)));
    }
  } catch {
    // 本地设置损坏时使用默认值
  }
};

const persistSettings = () => {
  try {
    localStorage.setItem(
      SETTINGS_STORAGE_KEY,
      JSON.stringify({
        autoLock: settingsForm.value.autoLock,
        lockSeconds: settingsForm.value.lockSeconds,
      })
    );
  } catch {
    // localStorage 不可用时忽略
  }
};

/// 把前端设置的锁定时长同步给后端会话（后端在超时后会丢弃解密密钥）
const syncSessionTimeout = async () => {
  if (!passwordsLoaded.value) return;
  try {
    await authApi.setSessionTimeout(settingsForm.value.lockSeconds);
  } catch {
    // 非致命：后端保留默认 30 分钟
  }
};

const markActivity = () => {
  lastActivityAt = Date.now();
};

const stopIdleWatch = () => {
  if (idleTimer !== undefined) {
    clearInterval(idleTimer);
    idleTimer = undefined;
  }
};

/// 锁定/退出时统一清空所有可能持有明文或敏感上下文的界面状态。
/// 新增任何明文相关的 ref，必须在这里登记，否则锁定时会残留。
const clearSensitiveState = () => {
  masterPassword.value = '';
  oldMasterPassword.value = '';
  newMasterPassword.value = '';
  confirmMasterPassword.value = '';
  exportPassword.value = '';
  exportPasswordConfirm.value = '';
  exportCandidates.value = [];
  exportSelectedIds.value = [];
  importForm.value = { file: null, fileName: '' };
  auditLogs.value = [];
  expiringPasswords.value = [];
  integrityReport.value = null;
  passwordPolicy.value = null;
  authError.value = '';
  showSecurityDialog.value = false;
  showExportDialog.value = false;
  showImportDialog.value = false;
  showBackupDialog.value = false;
  showSettingsDialog.value = false;
};

/// 锁定保险库：卸载 PasswordManager（销毁其内部明文）并清空界面状态
const lockVault = async (reason = '保险库已锁定') => {
  if (!passwordsLoaded.value) return;
  stopIdleWatch();
  // 组件卸载会释放它持有的条目列表与明文
  passwordsLoaded.value = false;
  clearSensitiveState();
  try {
    await authApi.logout();
  } catch {
    // 后端可能已因空闲超时自行锁定
  }
  authMode.value = 'unlock';
  showAuth.value = true;
  notificationApi.showNotification(reason, 'warning');
};

const checkIdle = () => {
  if (!passwordsLoaded.value || !settingsForm.value.autoLock) return;
  if (Date.now() - lastActivityAt >= settingsForm.value.lockSeconds * 1000) {
    lockVault(`已闲置 ${settingsForm.value.lockSeconds} 秒，保险库已自动锁定`);
  }
};

const startIdleWatch = () => {
  stopIdleWatch();
  lastActivityAt = Date.now();
  idleTimer = window.setInterval(checkIdle, IDLE_CHECK_INTERVAL_MS);
};

const onBackendLocked = () => {
  lockVault('会话已超时，保险库已锁定');
};

const openSecurityOverview = async () => {
  showSecurityDialog.value = true;
  securityLoading.value = true;
  try {
    const [policy, expiring, logs] = await Promise.all([
      passwordApi.getPasswordPolicy(),
      passwordApi.getExpiringPasswords(30),
      passwordApi.getAuditLogs(),
    ]);
    passwordPolicy.value = policy;
    expiringPasswords.value = expiring;
    auditLogs.value = logs;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notificationApi.showNotification('加载安全概览失败：' + msg, 'error');
  } finally {
    securityLoading.value = false;
  }
};

const runIntegrityCheck = async () => {
  integrityLoading.value = true;
  try {
    integrityReport.value = await passwordApi.checkIntegrity();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notificationApi.showNotification('完整性检查失败：' + msg, 'error');
  } finally {
    integrityLoading.value = false;
  }
};

const openExportDialog = async () => {
  exportPassword.value = '';
  exportPasswordConfirm.value = '';
  exportCandidates.value = [];
  exportSelectedIds.value = [];
  showExportDialog.value = true;
  try {
    exportCandidates.value = await passwordApi.getPasswords();
    await nextTick();
    // 默认全选，用户再按需取消
    if (exportCandidates.value.length > 0) {
      exportTableRef.value?.toggleAllSelection();
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    ElMessage.error('加载条目失败：' + msg);
  }
};

const onExportSelectionChange = (rows: PasswordEntry[]) => {
  exportSelectedIds.value = rows
    .map((row) => row.id)
    .filter((id): id is number => typeof id === 'number');
  // 勾选完条目后直接聚焦导出密码输入框
  nextTick(() => exportPasswordInputRef.value?.focus());
};

const selectAllExports = () => {
  exportTableRef.value?.toggleAllSelection();
};

const clearExportSelection = () => {
  exportTableRef.value?.clearSelection();
};

const handleEncryptedExport = async () => {
  const pwd = exportPassword.value;
  const confirm = exportPasswordConfirm.value;
  if (!pwd || pwd.length < 8) {
    notificationApi.showNotification('导出密码至少 8 位', 'warning');
    return;
  }
  if (pwd !== confirm) {
    notificationApi.showNotification('两次输入的导出密码不一致', 'warning');
    return;
  }
  if (exportSelectedIds.value.length === 0) {
    ElMessage.warning('请至少选择一条要导出的条目');
    return;
  }
  exporting.value = true;
  try {
    const content = await importExportApi.exportEncrypted(pwd, exportSelectedIds.value);
    const path = await importExportApi.saveFile(
      `securepass-export-${Date.now()}.spb`,
      content
    );
    ElMessage.success('已保存到 ' + path);
    showExportDialog.value = false;
    exportPassword.value = '';
    exportPasswordConfirm.value = '';
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    ElMessage.error('加密导出失败：' + msg);
  } finally {
    exporting.value = false;
  }
};

const handleSpbFileChange = (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0] ?? null;
  importForm.value.file = file;
  importForm.value.fileName = file?.name ?? '';
  // 清空 input，保证再次选同一个文件也会触发 change
  input.value = '';
};

const handleEncryptedImport = async () => {
  if (!importForm.value.file) {
    notificationApi.showNotification('请先选择 .spb 文件', 'warning');
    return;
  }
  let pwd = '';
  try {
    const res = await ElMessageBox.prompt('请输入该备份的导出密码', '解密备份', {
      inputType: 'password',
      confirmButtonText: '导入',
      cancelButtonText: '取消',
    });
    pwd = res.value;
  } catch (e) {
    return;
  }
  if (!pwd) {
    notificationApi.showNotification('导出密码不能为空', 'warning');
    return;
  }
  importing.value = true;
  try {
    const text = await importForm.value.file.text();
    const result = await importExportApi.importEncrypted(text, pwd);
    const summary =
      '导入完成：新增 ' + result.imported_count +
      '，跳过 ' + result.skipped_count +
      '，失败 ' + result.error_count;
    // 全部跳过通常是因为这些条目已在库里，提示得更明确一点
    const hint =
      result.imported_count === 0 && result.skipped_count > 0 ? '（条目已存在，无需重复导入）' : '';
    const level = result.error_count > 0 || result.imported_count === 0 ? 'warning' : 'success';
    notificationApi.showNotification(summary + hint, level);
    importForm.value = { file: null, fileName: '' };
    showImportDialog.value = false;
    // 强制重新挂载 PasswordManager，从后端重新拉取列表
    vaultVersion.value += 1;
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    notificationApi.showNotification('导入失败：' + msg, 'error');
  } finally {
    importing.value = false;
  }
};

const createBackup = async () => {
  try {
    const content = await importExportApi.exportBackup();
    const path = await importExportApi.saveFile(
      `securepass-backup-${Date.now()}.json`,
      content
    );
    ElMessage.success('备份已保存到 ' + path);
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    ElMessage.error('创建备份失败：' + msg);
  }
};

const showBackupFileSelect = () => {
  if (backupFileInput.value) {
    backupFileInput.value.click();
  }
};

const handleBackupFile = async (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files && input.files[0];
  if (!file) return;
  try {
    const data = await file.text();
    const { value } = await ElMessageBox.prompt('请输入创建该备份时使用的主密码', '恢复备份', {
      confirmButtonText: '恢复',
      cancelButtonText: '取消',
      inputType: 'password',
    });
    if (!value) {
      notificationApi.showNotification('请输入备份主密码', 'warning');
      return;
    }
    const result = await importExportApi.restoreBackup(data, value);
    notificationApi.showNotification(
      '恢复完成：新增 ' + result.imported_count + '，跳过 ' + result.skipped_count + '，失败 ' + result.error_count,
      result.error_count > 0 ? 'warning' : 'success'
    );
    showBackupDialog.value = false;
    passwordsLoaded.value = true;
    vaultVersion.value += 1;
  } catch (error) {
    if (error === 'cancel' || error === 'close') return;
    const msg = error instanceof Error ? error.message : String(error);
    notificationApi.showNotification('恢复备份失败：' + msg, 'error');
  } finally {
    input.value = '';
  }
};

/// 修改主密码：验证旧密码后由后端用新密钥重加密全部数据
const changeMasterPassword = async () => {
  if (!oldMasterPassword.value) {
    notificationApi.showNotification('请输入当前主密码', 'warning');
    return;
  }
  const missing = newMasterPasswordRules.value.filter((r) => !r.ok).map((r) => r.label);
  if (missing.length > 0) {
    notificationApi.showNotification('新主密码强度不足，需满足：' + missing.join('、'), 'warning');
    return;
  }
  if (newMasterPassword.value !== confirmMasterPassword.value) {
    notificationApi.showNotification('两次输入的新主密码不一致', 'warning');
    return;
  }
  if (newMasterPassword.value === oldMasterPassword.value) {
    notificationApi.showNotification('新主密码不能与当前主密码相同', 'warning');
    return;
  }
  changingPassword.value = true;
  try {
    await authApi.changePassword(oldMasterPassword.value, newMasterPassword.value);
    oldMasterPassword.value = '';
    newMasterPassword.value = '';
    confirmMasterPassword.value = '';
    notificationApi.showNotification('主密码已修改，请牢记新密码', 'success');
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    notificationApi.showNotification('修改主密码失败：' + msg, 'error');
  } finally {
    changingPassword.value = false;
  }
};

const saveSettings = async () => {
  savingSettings.value = true;
  try {
    persistSettings();
    await syncSessionTimeout();
    if (passwordsLoaded.value) startIdleWatch();
    notificationApi.showNotification('设置已保存', 'success');
    showSettingsDialog.value = false;
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    notificationApi.showNotification('设置保存失败：' + msg, 'error');
  } finally {
    savingSettings.value = false;
  }
};

// 生命周期
onMounted(async () => {
  loadSettings();
  // 任何用户活动都重置空闲计时
  ACTIVITY_EVENTS.forEach((ev) =>
    window.addEventListener(ev, markActivity, { passive: true })
  );
  window.addEventListener('securepass:locked', onBackendLocked);

  // 启动时检查是否已设置主密码：未设置则引导设置，已设置则要求解锁
  try {
    const set = await authApi.isSet();
    authMode.value = set ? 'unlock' : 'setup';
  } catch (e) {
    authMode.value = 'unlock';
  }
  showAuth.value = true;
});

onUnmounted(() => {
  stopIdleWatch();
  ACTIVITY_EVENTS.forEach((ev) => window.removeEventListener(ev, markActivity));
  window.removeEventListener('securepass:locked', onBackendLocked);
});

// 主密码设置 / 解锁提交
const submitAuth = async () => {
  authError.value = '';
  if (!masterPassword.value) {
    authError.value = '请输入主密码';
    return;
  }
  if (authMode.value === 'setup') {
    const missing = masterPasswordRules.value.filter((r) => !r.ok).map((r) => r.label);
    if (missing.length > 0) {
      authError.value = '主密码强度不足，需满足：' + missing.join('、');
      return;
    }
  }
  authLoading.value = true;
  try {
    if (authMode.value === 'setup') {
      await authApi.setup(masterPassword.value);
      notificationApi.showNotification('主密码设置成功', 'success');
    } else {
      await authApi.login(masterPassword.value);
      notificationApi.showNotification('保险库已解锁', 'success');
    }
    masterPassword.value = '';
    showAuth.value = false;
    passwordsLoaded.value = true;
    await syncSessionTimeout();
    startIdleWatch();
  } catch (e: any) {
    authError.value = e?.message || '操作失败，请重试';
  } finally {
    authLoading.value = false;
  }
};
</script>

<style scoped>
.app {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.app-container {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-bottom: 1px solid #e4e7ed;
  padding: 0;
}

.header-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
  height: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo h1 {
  margin: 0;
  font-size: 24px;
  color: #303133;
}

.subtitle {
  color: #909399;
  font-size: 14px;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.main {
  flex: 1;
  padding: 20px;
  background: rgba(255, 255, 255, 0.1);
}

.content-wrapper {
  max-width: 1200px;
  margin: 0 auto;
}

.welcome-section {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 400px;
}

.welcome-card {
  max-width: 600px;
  text-align: center;
}

.welcome-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
}

.welcome-icon {
  font-size: 64px;
}

.welcome-text h2 {
  margin: 0;
  color: #303133;
  font-size: 28px;
}

.welcome-text p {
  margin: 0;
  color: #909399;
  font-size: 16px;
}

.welcome-actions {
  display: flex;
  gap: 15px;
}

.export-options {
  margin-bottom: 20px;
}

.settings-hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}

.spb-picker {
  display: flex;
  align-items: center;
  gap: 10px;
}

.spb-name {
  color: #303133;
  font-size: 13px;
  word-break: break-all;
}

.spb-hint {
  color: #909399;
  font-size: 12px;
}

.export-select {
  margin-top: 16px;
}

.export-select-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-size: 13px;
  color: #606266;
}

.export-select-actions {
  display: flex;
  gap: 8px;
}

.export-info {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  margin-bottom: 20px;
}

.export-info p {
  margin: 5px 0;
  color: #606266;
}

.import-options {
  margin-bottom: 20px;
}

.backup-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.backup-info {
  color: #606266;
}

.backup-info p {
  margin: 10px 0 5px 0;
}

.backup-info ul {
  margin: 5px 0;
  padding-left: 20px;
}

.change-master-password {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.change-master-password .el-button {
  align-self: flex-start;
}

.password-strength {
  margin-top: 8px;
}

.strength-text {
  margin-left: 10px;
  font-size: 12px;
}

.strength-rules {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 2px 12px;
}

.strength-rules li {
  font-size: 12px;
  line-height: 1.6;
}

.strength-rules li.ok {
  color: #67c23a;
}

.strength-rules li.bad {
  color: #c0c4cc;
}

.weak {
  color: #f56c6c;
}

.medium {
  color: #e6a23c;
}

.strong {
  color: #67c23a;
}

.footer {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-top: 1px solid #e4e7ed;
  padding: 20px 0;
}

.footer-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
  text-align: center;
  color: #909399;
}

.footer-divider {
  margin: 0 10px;
}
</style>