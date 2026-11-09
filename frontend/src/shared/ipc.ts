// 不依赖任何 UI 库的 IPC 基础封装，启动器与设置窗共用。

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const tauri = (window as any).__TAURI__;
  const fn = tauri?.core?.invoke ?? tauri?.invoke;
  if (!fn) return Promise.reject(new Error('Tauri 运行时不可用'));
  return fn(cmd, args) as Promise<T>;
}

export async function request<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const res = await invoke<ApiResponse<T>>(cmd, args);
  if (!res.success) {
    const message = res.error || '操作失败';
    if (message.includes('保险库已锁定')) {
      window.dispatchEvent(new Event('vault-locked'));
    }
    throw new Error(message);
  }
  return res.data as T;
}
