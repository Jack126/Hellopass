# SecurePass 开发 TODO

> 最后同步：2026-10-20。历史阶段的重复记录已清理，本文件只保留当前状态与未完成项。

## ✅ 已完成
- 加密：AES-256-GCM；PBKDF2-HMAC-SHA256 600,000；salt 持久化 + 验证令牌；改密 rekey
- 数据库：SQLite；密码/备注加密存储；CRUD；数据库文件权限 0600
- 会话安全：30 分钟空闲超时；5 次失败锁定 30 分钟；解锁/锁定/失败写审计
- 审计日志：unlock/lock/add/update/delete/change_master_password 落库；get_audit_logs
- 密码策略：强度评估；过期（默认 90 天）；历史 5 条防复用
- 导入导出：JSON/CSV 导出；导入自动识别 + 去重统计
- Tauri 2：VaultRuntime + Mutex；19 个命令；程序化创建窗口 + 托盘；配置文件迁移 v2；图标与 capabilities
- 前端：主密码设置/解锁；列表/增删改/搜索/分类；导入/导出；安全概览（策略/过期/审计）
- 工程：无头单测 10 个通过；securepass bin 0 警告

## ⏳ 未完成（按处理顺序）

### 1. 真机与打包（当前无头环境阻塞）
- [ ] 在带 webkit2gtk-4.1 / libdbus-1-dev 的机器编译 securepass_tauri，并 cargo tauri dev 验证窗口/托盘/命令
- [ ] cargo tauri build 跨平台打包（Windows / macOS / Linux）

### 2. 备份加密 / 恢复 ✅
- [x] 后端：自包含加密备份包（version + iterations + salt + 密文），可跨安装用同一主密码恢复
- [x] Tauri 命令 export_backup / restore_backup（恢复需提供备份主密码）
- [x] 前端备份弹窗接线（下载加密备份 / 选择文件 + 输入备份密码恢复）
- [x] 单测：备份导出 → 恢复 roundtrip

### 3. 审计日志保留策略 ✅
- [x] 解锁时自动清理超过 90 天的 audit_logs；另提供 purge_old_audit_logs(days) 手动接口与测试

### 4. 多选导出 ✅
- [x] 后端 export_selected(ids, format)（JSON/CSV）
- [x] 前端导出弹窗列出条目勾选 + 全选/清空；不选则导出全部

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
