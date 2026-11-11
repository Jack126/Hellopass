# Hellopass 安全审查报告

| 项目 | 内容 |
| --- | --- |
| 审查对象 | `projects/passwd`（Rust + Tauri 2 跨平台桌面密码管理器 Hellopass） |
| 提交 | `d0c8d613` |
| 审查日期 | 2026-11-11 |
| 加密基线 | AES-256-GCM + PBKDF2-HMAC-SHA256（600,000 次迭代，32 字节盐） |
| 结论 | 加密方案本身扎实；**未发现"严重"级漏洞**。问题集中在能力配置错误、锁定时前端状态同步失效、安全控制可靠性不足三类 |

## 审查范围与方法

- 后端：`src/security/{crypto,master_password,key_management,policies}.rs`、`src/database/{mod,operations}.rs`、`src/api/{runtime,dto}.rs`、`src/main_tauri.rs`、`src/{main,error}.rs`、`src/utils/log.rs`
- 前端：`frontend/src/launcher/{Launcher.vue,ipc.ts,fuzzy.ts}`、`frontend/src/settings/SettingsApp.vue`、`frontend/src/shared/ipc.ts`
- 配置与依赖：`tauri.conf.json`、`capabilities/default.json`、`Cargo.toml`、`frontend/package.json`
- 交叉验证：本机 cargo registry 中的 `tauri-2.12.4`、`tauri-utils-2.10.2`、`pbkdf2-0.11.0`、`libsqlite3-sys-0.26.0` 源码
- 只读核对：本机残留数据 `~/.local/share/com.securepass.app/`、仓库根 `hellopass.db`
- 未做动态运行测试（无构建产物）

---

## 高危

### H1. capability 指向不存在的窗口，插件事件系统整体失效，"锁定通知"链路完全断裂

**位置**：`capabilities/default.json:4-6`（`windows: ["main"]`）。实际窗口标签为 `launcher`（`src/main_tauri.rs:934`）与 `settings`（`src/main_tauri.rs:697`）——**全项目不存在 `main` 窗口**。

**机制（已用 tauri-2.12.4 源码核实）**

- `RuntimeAuthority::resolve_access`（`tauri-2.12.4/src/ipc/authority.rs:462-485`）按窗口 / webview 标签过滤，过滤结果为空即返回 `None`；
- `tauri-2.12.4/src/webview/mod.rs:2080` 对 `plugin:*` 命令强制要求 ACL 命中，否则拒绝：

  ```rust
  if (plugin_command.is_some() || has_app_acl_manifest || !is_local) && invoke.acl.is_none() {
      reject(...)
  }
  ```

- 本项目 `capabilities/` 下只有 `default.json`，无 `__app-acl__` manifest，`has_app_manifest()` 恒为 false（`authority.rs:132`）→ **应用自定义命令被豁免，插件命令全部被拒**。

**后果**

1. `window.__TAURI__.event.listen()`（`shared/ipc.ts:22`、`launcher/ipc.ts:63`）在两个窗口上都抛 `Command plugin:event|listen not allowed by ACL`。
2. `Launcher.vue:538` 的 `await listen('launcher-shown', …)` 抛异常，**`onMounted` 在此中断**：`launcher-hidden`、`vault-locked` 两个清理监听从未注册，`onShown()`（决定 setup/unlock/search 初始态并加载列表）也从未执行。
3. `SettingsApp.vue:745` 同样抛异常，其 `onMounted` 末尾的 `vault_status` + `loadStats/loadAudit/loadExportItems` 不再执行，设置窗首次打开可能是未加载状态（仅靠 focus 事件补救）。
4. 备用路径同样是死代码：`shared/ipc.ts:21` 在"命令因锁定失败"时 `window.dispatchEvent(new Event('vault-locked'))`，但两个组件都**没有** `addEventListener('vault-locked', …)`。

**安全影响**：锁定（热键 / 超时 / 退出）后前端收不到任何通知，`revealed` 明文与 `detail` 继续留在 JS 堆与界面状态里，只能等 `window` focus / `visibilitychange` → `status()` 轮询才清理（`Launcher.vue:534`、`SettingsApp.vue:744`）；启动器失焦隐藏时 `revealed` 也不清空，下次唤醒会**未经重新认证再次显示上次明文**。

**修复**

1. `capabilities/default.json` 的 `windows` 改为 `["launcher", "settings"]`（或拆成两条 capability），确认 `core:default` 覆盖 `core:event:allow-listen`；
2. 两个组件补 `window.addEventListener('vault-locked', …)` 作为插件事件失效时的兜底；
3. `hide_launcher` / `lock` 时前端本地清理敏感状态，不依赖事件。

**复现**：在两个窗口的 DevTools 控制台执行 `window.__TAURI__.event.listen('x', () => {})`，返回 `Command plugin:event|listen not allowed by ACL`。

---

### H2. 完整性基线被应用自身的审计日志写入破坏（每次解锁必误报），无基线时又静默通过

**位置**：`src/api/runtime.rs:161`（每次 `unlock` 调 `verify_baseline`）、`:171-192`（无论成败都写 `audit_logs`）、`:228-238`（`lock` 时 `write_baseline`）、`:400-431`（基线 = **整个数据库文件的 SHA-256**）。

**机制**：数据库为 `journal_mode=delete`（实测仓库根 `hellopass.db`），任何 INSERT 都会改变主库文件；`unlock` 每次至少写一条审计 → 下一次 `unlock` 读到的基线与当前文件必然不等。

**后果**

- 只要基线存在，**每次解锁都报"数据库文件完整性校验失败"**，设置页据此提示"重建基线"；重建后下一次解锁同样误报，安全控制形同虚设并持续误导用户。
- 本机实测：仓库根 `hellopass.db` 旁边**没有** `.integrity` 文件 → `verify_baseline` 直接返回 `None`，校验完全空转。
- 附带：`.integrity` 用 `std::fs::write` 默认权限（通常 0644），且明文存哈希（无密钥），既不能防篡改也不保密。

**修复**

- 基线只覆盖受保护表内容的聚合哈希，或显式排除 `audit_logs`；
- `.integrity` 文件改 0600；
- 无基线时 UI 明确提示"基线缺失"，而不是静默通过。

---

### H3. 应用标识更名导致主密码配置失联，且 `setup` 可在旧库上直接覆盖初始化（数据不可访问）

**位置**：`tauri.conf.json:5`（`com.hellopass.app`）vs 本机残留 `~/.local/share/com.securepass.app/config/master_password_config.json`（提交 `a7f6330` 由 `com.securepass.app` 改名而来）；`try_setup`（`runtime.rs:126-137`）只看配置文件是否存在，不检查数据库里是否已有加密数据。

**后果**：老用户升级后 `is_set()` 返回 false，应用引导"设置新主密码"；`setup` 用新密码写配置并打开旧库（`CREATE TABLE IF NOT EXISTS` + 迁移，不动已有行），随后 `unlock` 用新令牌验证通过，但**历史条目全部用旧密钥加密 → 解密全部失败**（`get_passwords` 一损全损；`list_entry_summaries` 仍列出条目，`detail`/`reveal` 报错）。用户看到"条目都在却都打不开"，无提示、无回滚。

**本机证据**：`~/.local/share/com.hellopass.app` 目录不存在，旧配置在 `com.securepass.app` 下；仓库根 `hellopass.db` 含 100 条 `passwords`（2026-11-10 创建），其主密码配置属于上一标识。

**修复**

- 升级时搬迁 / 迁移旧标识目录（`com.securepass.app/config` → 新目录）；
- 或 `setup` 前检测"DB 已有 `passwords` 行但无配置"并强制告警；
- 至少补充文档说明恢复步骤。

---

## 中危

### M1. 无应用级 ACL manifest + `withGlobalTauri: true`，43 个命令对任意 webview JS 可调用

**位置**：`tauri.conf.json:13`；`src/main_tauri.rs:1060-1103`（`generate_handler!` 注册了 `unlock` / `get_passwords` / `export_backup` / `restore_backup` / `save_export_file` / `set_session_timeout` 等）。本地内容 + 无 `__app-acl__` → ACL 分支不生效（`webview/mod.rs:2080`）。

**现状缓解**：CSP `script-src 'self'`、全项目无 `v-html` / `innerHTML` / `eval`、无远程导航。**当前不可直接利用**，但未来任何 XSS（或 dev server / 依赖被劫持）都会变成完整保险库访问。

**修复**：新增 app 级 capability（`identifier: "__app-acl__"`）按窗口 allowlist 命令；生产构建关闭 `withGlobalTauri`，改用受控的模块注入。

### M2. `change_master_password` 不受失败锁定约束，可无限在线验证主密码

**位置**：`runtime.rs:243-252` → `:257`（`old_mgr.verify_password(old_password)?`）全程不读 / 不写 `failed_attempts`、`lockout_until`；只有成功才写审计（`:297`）。

**影响**：在已解锁会话内可以每次约 200ms（PBKDF2 600k）的速度无限验证主密码猜测，绕过 `unlock` 的 5 次 / 30 分钟锁定。前提虽是会话已解锁，但它把"验证一个主密码猜测"变成了无成本在线服务。

**修复**：`change_password` 纳入同一套失败计数 / 锁定 / 审计逻辑（失败也写审计）。

### M3. 默认"永不自动锁定"，且超时后前端不清理

**位置**：`runtime.rs:68`（`session_timeout: Duration::zero()`）；前端 `localStorage.sp.autoLockMinutes` 默认 0（`Launcher.vue:530`、`SettingsApp.vue:741`）。

**影响**：默认安装即"解锁后永久保持"，而启动器形态下用户很少打开设置窗修改；配合 H1，超时后界面继续显示明文 / 列表，直到焦点事件触发 `status()` 才清理。

**修复**：默认 5 或 15 分钟；超时路径主动 `emit("vault-locked")`（并修好监听）。

### M4. 导出 / 备份格式自描述 KDF 参数且导入不做下限校验（可降级 KDF 加速离线破解）

**位置**：`runtime.rs:742`（导出密码仅要求 ≥8 位，无复杂度要求）；`try_import_encrypted`（`:800-812`）把包里的 `iterations` 直接传给 `SecureCryptoEngine::new_with_iterations`，而 `crypto.rs:103` 对 `iterations` **不做任何校验**（`pbkdf2 0.11` 对 0 / 1 不报错，只执行一次 HMAC）。

**影响**：攻击者拿到加密导出 / 备份文件后，把 `iterations` 改成 1 即可让离线破解导出密码快约 60 万倍；导出文件包含全部明文，8 位导出密码在离线场景下本就脆弱。

**修复**：导出密码复用 `password_is_strong`（≥12 位 + 复杂度）；导入时强制 `iterations ≥ 100_000` 并校验 salt 长度。

### M5. 主密钥内存清零不完整

**位置**：`crypto.rs:44-51`（`master_key: [u8;32]` 非 `Zeroizing`）、`:73-76` / `:103-107`（派生临时缓冲）、`:188`（仅 `Drop` 清零）；`Clone` 会复制密钥（用于 `runtime.rs:196/258/270`）。

**影响**：崩溃转储 / 换页 / 核心转储中可能残留密钥。属加固项，不是可直接利用的漏洞。

**修复**：字段改 `Zeroizing<[u8; 32]>`，派生输出用 `Zeroizing`，避免不必要的 `Clone`（改用引用或 `from_engine`）。

### M6. 明文在前端驻留，reveal 后无自动隐藏

**位置**：`Launcher.vue:150`（`revealed` ref），Alt 松开才清（`onKeyUp`），无 TTL；`detail.password` 同理。

**影响**：明文长时间留在屏幕与 JS 堆中；配合 H1，锁定后的清除时机被动依赖焦点轮询。

**修复**：reveal 后 N 秒自动隐藏；窗口隐藏 / 锁定时立即本地清空（不等事件）。

---

## 低危 / 加固

- **L1** 权限与锁定状态文件未收紧：`lockout_state.json`（`runtime.rs:88-99`）、`ui.json`、`hotkey.json`、`<db>.integrity` 均用 `std::fs::write` 默认权限；删除 `lockout_state.json` 即可重置 5 次失败计数与 30 分钟锁定。建议 0600，并把锁定状态并入 DB 或做签名。（本地攻击者本可改库，实际风险有限。）
- **L2** `is_set()` 仅凭配置文件存在判断（`runtime.rs:117-119`）：空 / 截断文件会让应用既不能 `setup` 也不能 `unlock`（"存储文件损坏"），构成浅色 DoS，应给出明确恢复路径。
- **L3** 审计缺口：读取类命令（`list_entry_summaries` / `get_entry_detail` / `vault_status`）不写审计；锁定期内被拒的 `unlock` 不写审计；`change_master_password` 失败不写审计。密码管理器通常需要"谁看过这条记录"的追溯能力。
- **L4** `get_password_stats` 返回桩数据（`tag_counts` 恒空、`recent_access_count` 恒 0），安全概览会给出误导性结论（"高风险 0 条"）。属正确性问题，但影响安全决策。
- **L5** `check_hotkey_available` 恒返回成功，热键冲突检测形同虚设。
- **L6** 未启用 macOS 屏幕内容保护（`set_content_protected`），reveal 明文期间截图 / 录屏可捕获。
- **L7** CSP `style-src 'self' 'unsafe-inline'`（`tauri.conf.json:16`）：Vue SFC 需要内联样式，可用 nonce / hash 收敛；当前无 XSS sink，风险低。
- **L8** 文档与实现不一致：`security-architecture.md` 写 100,000 次迭代，代码与本机配置均为 600,000；`main.rs` 的 CLI 用 `app_data_dir/passwords.db`、GUI 用 `hellopass.db`，两者操作不同数据库（`seed-demo` 种的数据 GUI 看不到）；`VERIFY_TOKEN` 仍为旧品牌名（改动会导致已有库无法验证，属兼容性约束，建议注释说明）。

---

## 已核实无问题（不计入问题项）

- **加密**：AES-256-GCM，每次加密随机 96-bit nonce，`nonce‖ct‖tag` 认证加密；密钥只存内存，数据库只存密文；PBKDF2-HMAC-SHA256 600k + 32 字节随机盐；验证令牌方案始终执行完整 KDF，无定时侧信道，错误统一为"主密码错误"。
- **剪贴板**：明文只在 Rust 侧写入（`copy_secret`，`main_tauri.rs:369-391`），30 秒摘要校验自动清理（`:325-350`），锁定 / 退出时清理，且不会误清用户后来复制的新内容。
- **SQL**：全部参数化（`rusqlite::params!`），无字符串拼接；`DELETE … LIMIT` 均写在子查询内，不依赖 `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`。
- **前端**：无 `v-html` / `innerHTML` / `eval` / `document.write`，全文本插值；搜索算法无 ReDoS。
- **其他**：数据库文件 0600（unix）+ `PRAGMA integrity_check`；审计日志不含密码 / 主密码 / 明文；导出文件名经 `file_name()` 消毒并写 0600。

---

## 建议修复顺序

1. **H1**（capability 窗口名 + 锁定清理兜底）——改动最小、直接影响锁定安全语义。
2. **H2**（基线语义）——安全控制当前完全不可信。
3. **H3**（标识迁移 / 旧库检测）——防止既有数据不可访问。
4. **M1、M2、M3**——ACL 纵深、限速一致性、默认自动锁定。
5. 其余按迭代排期。

---

## 附：关键核实点索引

| 核实项 | 结论 | 证据位置 |
| --- | --- | --- |
| `plugin:*` 命令是否受 ACL 约束 | 是（无需 app manifest） | `tauri-2.12.4/src/webview/mod.rs:2080` |
| app 自定义命令是否受 ACL 约束 | 否（本地来源 + 无 `__app-acl__`） | `tauri-2.12.4/src/ipc/authority.rs:132`、`webview/mod.rs:2080` |
| capability 窗口过滤为空时 | 返回 `None` → 拒绝 | `authority.rs:462-485` |
| 事件 API 底层命令 | `plugin:event\|listen` | `tauri-2.12.4/scripts/bundle.global.js` |
| `pbkdf2` 对 `rounds < 2` 的处理 | 静默只执行一次 HMAC | `pbkdf2-0.11.0/src/lib.rs` |
| SQLite 是否启用 `UPDATE_DELETE_LIMIT` | 未启用（但代码用子查询写法，不受影响） | `libsqlite3-sys-0.26.0/build.rs`、`operations.rs:359/427` |
| 本机数据库日志模式 | `delete`（每次 INSERT 改动主库文件） | `hellopass.db` 实测 |
| 本机 `.integrity` 基线文件 | 不存在 → 校验空转 | 仓库根目录 |
| 本机主密码配置位置 | `com.securepass.app/config/`（旧标识） | `~/.local/share/` |
