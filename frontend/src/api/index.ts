// SecurePass 前端 API 客户端（Tauri 集成）
//
// 通过 window.__TAURI__.invoke 调用 Rust 后端命令（tauri.conf.json 已开启
// withGlobalTauri，因此无需额外安装 @tauri-apps/api 即可使用全局 API）。
//
// 约定：每个 Rust 命令返回 ApiResponse<T>；本客户端将其规范化为
// 「成功则返回 data，失败则 reject」的 Promise，便于上层 try/catch 处理。

import { ElMessage } from 'element-plus';

const invoke = <T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  const tauri = (window as any).__TAURI__;
  // Tauri v2 的全局对象是 { core: { invoke }, ... }；兼容 v1 的 { invoke }
  const invokeFn = tauri?.core?.invoke ?? tauri?.invoke;
  if (!invokeFn) {
    return Promise.reject(new Error('Tauri 运行时不可用：请在桌面应用（tauri dev / tauri build）中运行'));
  }
  return invokeFn(cmd, args) as Promise<T>;
};

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

async function request<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const res = await invoke<ApiResponse<T>>(cmd, args);
  if (!res.success) {
    const message = res.error || '操作失败';
    // 后端会话已超时/被锁定时，通知 UI 卸载明文并回到解锁界面
    if (message.includes('保险库已锁定')) {
      window.dispatchEvent(new CustomEvent('securepass:locked'));
    }
    throw new Error(message);
  }
  return res.data as T;
}

// ===================== 密码管理 API =====================
export const passwordApi = {
  // 获取所有密码（明文仅在解锁会话内由后端返回）
  getPasswords: () => request<PasswordEntry[]>('get_passwords'),
  // 添加密码
  addPassword: (password: NewPassword) =>
    request<PasswordEntry>('add_password', { password }),
  // 更新密码（id 合并进 body）
  updatePassword: (id: number, password: UpdatePassword) =>
    request<PasswordEntry>('update_password', { password: { id, ...password } }),
  // 删除密码
  deletePassword: (id: number) => request<undefined>('delete_password', { id }),
  // 搜索密码
  searchPasswords: (query: string) =>
    request<PasswordEntry[]>('search_passwords', { query: { query } }),
  // 密码策略与安全
  getPasswordPolicy: () => request<PasswordPolicy>('get_password_policy'),
  getExpiringPasswords: (withinDays = 30) =>
    request<PasswordExpiryInfo[]>('get_expiring_passwords', { withinDays }),
  checkPasswordStrength: (password: string) =>
    request<PasswordStrengthResult>('check_password_strength', { password }),
  getAuditLogs: () => request<AuditLogEntry[]>('get_audit_logs'),
  checkIntegrity: () => request<IntegrityReport>('check_integrity'),
};

// ===================== 认证 / 保险库 API =====================
export const authApi = {
  // 是否已设置主密码
  isSet: () => request<boolean>('is_master_password_set'),
  // 首次设置主密码
  setup: (masterPassword: string) =>
    request<undefined>('setup_master_password', { masterPassword }),
  // 解锁保险库
  login: (masterPassword: string) =>
    request<undefined>('unlock', { masterPassword }),
  // 锁定保险库
  logout: () => request<undefined>('lock'),
  // 同步空闲超时（秒）到后端会话
  setSessionTimeout: (seconds: number) =>
    request<undefined>('set_session_timeout', { seconds }),
  // 修改主密码
  changePassword: (oldPassword: string, newPassword: string) =>
    request<undefined>('change_master_password', {
      oldPassword,
      newPassword,
    }),
  // 密码强度校验（调用后端策略评估）
  validatePassword: async (password: string): Promise<ApiResponse<PasswordStrengthResult>> => {
    try {
      const detail = await request<PasswordStrengthResult>('check_password_strength', { password });
      return { success: true, data: detail, timestamp: new Date().toISOString() };
    } catch (e) {
      return { success: false, error: (e as Error).message, timestamp: new Date().toISOString() };
    }
  },
};

// ===================== 导入导出 API =====================
export const importExportApi = {
  // 导入密码：后端自动识别 JSON/CSV，返回导入统计
  importPasswords: (data: string) =>
    request<ImportResult>('import_passwords', { data }),
  // 创建加密备份：返回自包含备份包（JSON 字符串）
  exportBackup: () => request<string>('export_backup'),
  exportEncrypted: (exportPassword: string, ids?: number[]) =>
    request<string>('export_encrypted', { exportPassword, ids: ids ?? null }),
  // 把导出内容写到系统「下载」目录（WebView 的 <a download> 在 WebKitGTK 下无效）
  saveFile: (filename: string, content: string) =>
    request<string>('save_export_file', { filename, content }),
  importEncrypted: (data: string, exportPassword: string) =>
    request<ImportResult>('import_encrypted', { data, exportPassword }),
  // 从加密备份恢复：data 为备份包内容，masterPassword 为创建备份时的主密码
  restoreBackup: (data: string, masterPassword: string) =>
    request<ImportResult>('restore_backup', { data, masterPassword }),
};

// ===================== 通知 API =====================
export const notificationApi = {
  // 统一走 Element Plus 的全局提示（之前只 console.log，用户看不到任何反馈）
  showNotification: (message: string, type: 'success' | 'error' | 'warning' = 'success') => {
    ElMessage({ message, type, duration: 3000, showClose: true });
  },
};

// ===================== 数据类型定义 =====================
export interface PasswordEntry {
  id?: number;
  site_name: string;
  site_url: string;
  username: string;
  encrypted_password: string;
  encrypted_notes?: string;
  /** 明文密码（仅在解锁会话内由后端解密返回） */
  password?: string;
  /** 明文备注 */
  notes?: string;
  created_at: string;
  updated_at: string;
  last_accessed?: string;
  access_count: number;
  tags: string[];
  category: string;
  priority: number;
  metadata?: string;
  checksum?: string;
}

export interface NewPassword {
  site_name: string;
  site_url: string;
  username: string;
  password: string;
  notes?: string;
  tags?: string[];
  category?: string;
  priority?: number;
}

export interface UpdatePassword {
  site_name?: string;
  site_url?: string;
  username?: string;
  password?: string;
  notes?: string;
  tags?: string[];
  category?: string;
  priority?: number;
}

export interface SearchQuery {
  query: string;
  category?: string;
  tags?: string[];
  limit?: number;
  offset?: number;
}

export interface PasswordStats {
  total_count: number;
  category_counts: Record<string, number>;
  tag_counts: Record<string, number>;
  recent_access_count: number;
}

export interface ImportResult {
  imported_count: number;
  skipped_count: number;
  updated_count: number;
  error_count: number;
  errors: string[];
}

export interface PasswordPolicy {
  max_age_days: number;
  history_limit: number;
}

export interface PasswordExpiryInfo {
  id?: number;
  site_name: string;
  days_until_expiry: number;
  expired: boolean;
}

export interface AuditLogEntry {
  id: number;
  timestamp: string;
  action: string;
  resource_type?: string;
  resource_id?: string;
  details?: string;
  success: boolean;
  error_message?: string;
}

export interface PasswordStrengthResult {
  is_valid: boolean;
  score: number;
  feedback: string[];
  suggestions: string[];
}

export interface IntegrityReport {
  sqlite_ok: boolean;
  db_checksum: string;
  db_size: number;
  config_checksum?: string;
  baseline_ok: boolean;
}

export interface LoginRequest {
  master_password: string;
  remember_me?: boolean;
}

export interface LoginResponse {
  success: boolean;
  session_id?: string;
  error?: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

// ===================== 错误处理 =====================
export const handleApiError = (error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error('API Error:', message);
  notificationApi.showNotification(message || '请求失败', 'error');
  throw error;
};

export default { passwordApi, authApi, importExportApi, notificationApi };
