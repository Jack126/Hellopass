# Hellopass 开发 TODO

> 最后同步：2026-11-09。
> 2026-11-09 起已改为「托盘 + 热键启动器」形态，下面「已完成」里关于主窗口/分类/30 秒自动锁的描述属于历史阶段，仅作参考。

## ✅ 已完成
- 加密：AES-256-GCM；PBKDF2-HMAC-SHA256 600,000；salt 持久化 + 验证令牌；改密 rekey
- 数据库：SQLite；密码/备注加密存储；CRUD；数据库文件权限 0600
- 会话安全：30 秒空闲超时（可调 10~300 秒）；5 次失败锁定 30 分钟；解锁/锁定/失败写审计
- 审计日志：unlock/lock/add/update/delete/change_master_password 落库；get_audit_logs
- 密码策略：强度评估；过期（默认 90 天）；历史 5 条防复用
- 导入导出：加密导出（.spb）；导入自动识别 + 去重统计（明文导出命令已移除）
- 会话：前端空闲自动锁定 + 手动锁定按钮，设置项落 localStorage 并同步后端超时
- Tauri 2：VaultRuntime + Mutex；24 个命令；程序化创建窗口 + 托盘；配置文件迁移 v2；图标与 capabilities
- 前端：主密码设置/解锁；列表/增删改/搜索/分类；导入/导出；安全概览（策略/过期/审计）
- 工程：无头单测 10 个通过；hellopass bin 0 警告

## ⏳ 未完成（按处理顺序）

### 0. 启动器窗口分辨率 / 缩放 ✅（2026-11-09 完成）
- [x] 设置 → 通用 增加「窗口大小」：自动 / 紧凑 / 标准 / 大，持久化到 config/ui.json，切换即时应用
- [x] 自动模式：读取鼠标所在显示器的可用尺寸与 DPI 缩放，按比例计算窗口宽高
  - 宽度建议 clamp 在 560~900：约为显示器逻辑宽度的 35%，4K/125% 下不至于过小
  - 高度随状态走：搜索 ~420、详情/编辑 ~560，按同一比例缩放
- [x] 修正 `resize_launcher` 硬编码宽度 680 的问题，改为按偏好 + 显示器计算
- [x] 多显示器：改为跟随鼠标所在显示器（`cursor_position` + `monitor_from_point`），窗口位置用物理像素计算
- [ ] 验收：1080p@100%、2K@125%、4K@150% 下弹窗不超出屏幕、字体不模糊、详情态高度足够
- [ ] 可选：结果列表行数随窗口高度自适应（当前固定最多 8 条）


> 实现：`src/utils/window_size.rs`（纯函数 + 单测）、`ui.json` 持久化、物理像素定位避免混合 DPI 偏移。

### 9. 固定格式 Excel / CSV 数据导入（后期再做）
- [ ] 支持从 **固定模板** 的 Excel（.xlsx）与 CSV 导入密码条目
  - 模板列固定，例如：网站名称 / URL / 用户名 / 密码 / 备注（列名与顺序先定一份规范，随文档发布）
  - CSV 按固定分隔与编码（UTF-8 with BOM）解析；Excel 解析为同样的内部行结构
  - 复用现有导入去重逻辑（站点名 + 用户名），返回新增/跳过统计
- [ ] 导入前展示模板说明 / 提供示例模板下载，避免用户手搓格式
- [ ] 仅解析已知列，多余列忽略；空行与缺列行跳过并计入告警
- 注意：与现有「加密 .spb 导入」并列，明文 Excel/CSV 导入属于便捷通道，**不做批量明文导出**（保持现有安全策略）

### 10. 一键清理 ✅（2026-11-12）
- [x] 所有密码一键清理，方便重新导入（设置 → 安全 → 清空所有记录，二次点击确认）
- [x] 导入覆盖而不是跳过（按 名称 + 网址 + 用户名 识别，同一站点可保留多个账号；未提供的备注/标签沿用原值）

### 11. 客户端
- [ ] apk 开发

## 历史阶段未完成项

### 1. 真机与打包（当前无头环境阻塞）
- [ ] 在带 webkit2gtk-4.1 / libdbus-1-dev 的机器编译 hellopass，并 cargo tauri dev 验证窗口/托盘/命令
- [ ] cargo tauri build 跨平台打包（Windows / macOS / Linux）

### 2. 备份加密 / 恢复 ✅
- [x] 后端：自包含加密备份包（version + iterations + salt + 密文），可跨安装用同一主密码恢复
- [x] Tauri 命令 export_backup / restore_backup（恢复需提供备份主密码）
- [x] 前端备份弹窗接线（下载加密备份 / 选择文件 + 输入备份密码恢复）
- [x] 单测：备份导出 → 恢复 roundtrip

### 3. 审计日志保留策略 ✅
- [x] 解锁时自动清理超过 90 天的 audit_logs；另提供 purge_old_audit_logs(days) 手动接口与测试

### 4. 导出（已改为仅加密导出）
- [x] 明文 JSON/CSV 导出命令（export_passwords / export_passwords_csv / export_selected）已移除
- [x] 统一走 export_encrypted：独立导出密码 + AES-256-GCM + 自包含 .spb 包

### 5. 密码生成器 ✅（固定 16 位）
- [x] 前端使用 crypto.getRandomValues 生成 16 位强随机密码（大小写+数字+特殊字符）
- [ ] 可配置长度/字符集（后续）

### 6. 完整性校验 ✅
- [x] 文件级完整性检查：SQLite integrity_check + 数据库/配置 SHA-256（安全概览可触发）
- [x] 自动运行时完整性校验：正常锁定写入 DB 文件基线；下次解锁比对，异常写日志并在 IntegrityReport.baseline_ok 体现（告警，不阻断）

### 7. 数据迁移 / 跨设备同步 ✅
- [x] 老版本数据迁移：`schema_meta` 版本记录 + 启动时补齐缺失列（ALTER TABLE）
- [x] 跨设备同步：加密备份包自包含 salt，可在另一安装恢复（手工同步）

### 8. 工程与文档
- [x] 统一跨模块错误类型：`src/error.rs::AppError` 已在运行时边界落地（setup/change_password/restore_backup 走 `AppResult`；`with_vault` 把 `OperationError` 转 `AppError` 并统一记日志）
- [x] 集成测试：`full_lifecycle_integration`（共 24 个测试通过）
- [x] 安全测试：密文篡改/错误密钥、错误主密码、错误备份密码
- [x] 跨平台 CI：`.github/workflows/ci.yml` 三平台矩阵（后端测试 + 前端构建 + Tauri 打包），待推送实际运行
- [x] 同步 PROJECT_SUMMARY.md / DEPLOYMENT.md
- [x] 同步 README.md / todo.md（2026-10-20）
