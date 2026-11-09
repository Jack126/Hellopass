import { describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

// 静态守护：新增敏感状态却忘了在 clearSensitiveState 里登记时，这里会失败。
const appPath = path.resolve(__dirname, '../App.vue');
const src = fs.readFileSync(appPath, 'utf8');

// 名字带这些关键字的 ref 视为"可能持有明文/密钥上下文"
const SENSITIVE_KEYWORDS = [
  'Password',
  'Export',
  'Import',
  'Audit',
  'Integrity',
  'Policy',
  'Expiring',
  'Secret',
];

// 纯 UI 状态：DOM 引用、加载/提交标志，不含明文或密钥
const NON_SENSITIVE = new Set([
  'exportTableRef',
  'exportPasswordInputRef',
  'spbFileInput',
  'passwordsLoaded',
  'exporting',
  'importing',
  'changingPassword',
  'integrityLoading',
  'securityLoading',
  'savingSettings',
]);

function sliceFrom(decl: string, endMarker: string): string {
  const start = src.indexOf(decl);
  if (start < 0) throw new Error('未找到: ' + decl);
  const end = src.indexOf(endMarker, start);
  return end < 0 ? src.slice(start) : src.slice(start, end);
}

describe('clearSensitiveState 静态守护', () => {
  it('函数存在，且 lockVault 调用了它', () => {
    const clearBody = sliceFrom('const clearSensitiveState = () => {', '/// 锁定保险库');
    expect(clearBody).toContain('masterPassword.value');
    expect(clearBody).toContain('exportPassword.value');

    const lockBody = sliceFrom('const lockVault = async', '/// 统计');
    expect(lockBody).toContain('clearSensitiveState()');
    expect(lockBody).toContain('passwordsLoaded.value = false');
  });

  it('所有敏感 ref 都被 clearSensitiveState 清空', () => {
    const clearBody = sliceFrom('const clearSensitiveState = () => {', '/// 锁定保险库');
    const refs = [...src.matchAll(/const\s+(\w+)\s*=\s*ref(?:<[^>]*>)?\(/g)].map(
      (m) => m[1]
    );
    const sensitive = [...new Set(refs)].filter(
      (name) =>
        !NON_SENSITIVE.has(name) &&
        SENSITIVE_KEYWORDS.some((kw) => name.toLowerCase().includes(kw.toLowerCase()))
    );

    // 防止正则失效导致"空集合永远通过"
    expect(sensitive.length).toBeGreaterThanOrEqual(8);

    const missing = sensitive.filter(
      (name) => !new RegExp(name + '\\.value\\s*=').test(clearBody)
    );
    expect(missing, '这些敏感状态没有在 clearSensitiveState 中清空').toEqual([]);
  });
});
