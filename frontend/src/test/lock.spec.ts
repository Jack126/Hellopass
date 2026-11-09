import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import ElementPlus from 'element-plus';
import App from '../App.vue';

const ok = (data: unknown) => ({
  success: true,
  data,
  error: null,
  timestamp: new Date().toISOString(),
});

const entry = {
  id: 1,
  site_name: 'GitHub',
  site_url: 'https://github.com',
  username: 'alice',
  encrypted_password: '',
  password: 'Secret#123',
  notes: 'note',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  last_accessed: null,
  access_count: 0,
  tags: ['work'],
  category: 'work',
  priority: 2,
};

let invoke: ReturnType<typeof vi.fn>;

const setValue = (el: HTMLInputElement, value: string) => {
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
};

const clickButton = (text: string) => {
  const btn = [...document.querySelectorAll('button')].find(
    (b) => (b.textContent || '').trim() === text
  ) as HTMLButtonElement | undefined;
  if (!btn) throw new Error('找不到按钮: ' + text);
  btn.click();
};

beforeEach(() => {
  invoke = vi.fn((cmd: string) => {
    switch (cmd) {
      case 'is_master_password_set':
        return Promise.resolve(ok(true));
      case 'get_passwords':
        return Promise.resolve(ok([entry]));
      case 'unlock':
      case 'lock':
      case 'set_session_timeout':
        return Promise.resolve(ok(undefined));
      default:
        return Promise.resolve(ok(null));
    }
  });
  (window as any).__TAURI__ = { core: { invoke } };
});

afterEach(() => {
  document.body.innerHTML = '';
  delete (window as any).__TAURI__;
  vi.restoreAllMocks();
});

async function unlockApp() {
  const wrapper = mount(App, {
    attachTo: document.body,
    global: { plugins: [ElementPlus] },
  });
  await flushPromises();

  const pwd = document.querySelector('input[type="password"]') as HTMLInputElement;
  expect(pwd).toBeTruthy();
  setValue(pwd, 'StrongPass123!');
  clickButton('解锁');
  await flushPromises();
  return wrapper;
}

describe('锁定时清理前端状态', () => {
  it('卸载密码列表并清空所有明文输入', async () => {
    const wrapper = await unlockApp();

    // 解锁后主界面应该挂载
    expect(document.querySelector('.password-manager')).toBeTruthy();

    // 打开导出弹窗，填入导出密码
    clickButton('导出密码');
    await flushPromises();
    const exportPwd = document.querySelector(
      'input[placeholder="设置导出密码（导入时需输入相同密码）"]'
    ) as HTMLInputElement;
    expect(exportPwd).toBeTruthy();
    setValue(exportPwd, 'Export#123456');

    // 打开设置，填入新主密码
    clickButton('设置');
    await flushPromises();
    const newPwd = document.querySelector(
      'input[placeholder="新主密码：至少 12 位，含大小写字母、数字与符号"]'
    ) as HTMLInputElement;
    expect(newPwd).toBeTruthy();
    setValue(newPwd, 'Another#123456');

    // 锁定
    clickButton('锁定');
    await flushPromises();

    // 1) 密码列表组件被卸载
    expect(document.querySelector('.password-manager')).toBeNull();
    // 2) 明文输入被清空
    expect(exportPwd.value).toBe('');
    expect(newPwd.value).toBe('');
    // 3) 回到解锁界面
    expect(document.body.textContent).toContain('解锁保险库');
    // 4) 后端确实收到了 lock
    expect(invoke).toHaveBeenCalledWith('lock', undefined);

    wrapper.unmount();
  });
});
