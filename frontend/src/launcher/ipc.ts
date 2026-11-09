// 启动器的 IPC 封装（基于 shared/ipc，不依赖 Element Plus）。
import { request } from '../shared/ipc';

export interface Summary {
  id: number;
  site_name: string;
  site_url: string;
  username: string;
  tags: string[];
  category: string;
  updated_at: string;
  last_accessed: string | null;
  access_count: number;
  pinyin_initials: string;
}

export interface EntryDetail {
  id: number;
  site_name: string;
  site_url: string;
  username: string;
  notes: string | null;
  tags: string[];
  category: string;
  created_at: string;
  updated_at: string;
  last_accessed: string | null;
  access_count: number;
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

export const api = {
  isSet: () => request<boolean>('is_master_password_set'),
  status: () => request<boolean>('vault_status'),
  setup: (masterPassword: string) => request<undefined>('setup_master_password', { masterPassword }),
  unlock: (masterPassword: string) => request<undefined>('unlock', { masterPassword }),
  lock: () => request<undefined>('lock'),
  summaries: () => request<Summary[]>('list_entry_summaries'),
  detail: (id: number) => request<EntryDetail>('get_entry_detail', { id }),
  reveal: (id: number) => request<string>('reveal_password', { id }),
  copy: (id: number, field: 'password' | 'username') => request<undefined>('copy_secret', { id, field }),
  add: (password: NewPassword) => request<unknown>('add_password', { password }),
  update: (password: Record<string, unknown>) => request<unknown>('update_password', { password }),
  remove: (id: number) => request<undefined>('delete_password', { id }),
  hide: () => request<undefined>('hide_launcher'),
  resize: (height: number) => request<undefined>('resize_launcher', { height }),
  suppressHide: (suppress: boolean) => request<undefined>('set_suppress_hide', { suppress }),
  setSessionTimeout: (seconds: number) => request<undefined>('set_session_timeout', { seconds }),
};

export async function listen<T = unknown>(
  event: string,
  handler: (payload: T) => void
): Promise<() => void> {
  const tauri = (window as any).__TAURI__;
  if (!tauri?.event?.listen) return () => {};
  const unlisten = await tauri.event.listen(event, (e: { payload: T }) => handler(e.payload));
  return typeof unlisten === 'function' ? unlisten : () => {};
}
