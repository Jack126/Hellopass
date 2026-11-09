# SecurePass 项目总结

## 概述
SecurePass 是基于 Vue 3 + Rust + Tauri 2 的跨平台桌面密码管理器。密码使用 AES-256-GCM 加密，主密钥由 PBKDF2-HMAC-SHA256（600,000 次）派生。

## 项目结构
```
passwd/
├── src/
│   ├── main.rs                 # securepass_cli：无头自测入口
│   ├── main_tauri.rs           # securepass：Tauri 2 GUI 入口
│   ├── error.rs                # 统一错误类型 AppError
│   ├── security/
│   │   ├── crypto.rs           # AES-256-GCM + PBKDF2
│   │   ├── master_password.rs  # 主密码/salt/验证令牌
│   │   ├── key_management.rs   # 密钥管理
│   │   └── policies.rs         # 密码强度评估
│   ├── database/
│   │   ├── mod.rs              # 初始化/迁移/schema 版本/统计/文件校验
│   │   ├── operations.rs       # CRUD/审计/历史/过期/备份加解密
│   │   └── schema.sql          # 建表脚本（文档）
│   ├── api/
│   │   ├── dto.rs              # ApiResponse + DTO
│   │   ├── runtime.rs          # VaultRuntime（无 Tauri 依赖）
│   │   └── mod.rs
│   └── utils/log.rs            # 统一日志
├── frontend/                   # Vue 3（无 UI 框架）
│   └── src/
│       ├── launcher/           # 热键搜索弹窗（Launcher.vue / fuzzy.ts）
│       ├── settings/           # 托盘打开的设置窗
│       └── shared/ipc.ts       # invoke 封装
├── icons/  assets/  capabilities/
├── tauri.conf.json             # Tauri 2 配置
└── Cargo.toml
```

## 已完成
- 加密与主密码：AES-256-GCM；PBKDF2 600k；salt 持久化；改密 rekey
- 数据库：SQLite；CRUD；审计；密码历史（5 条防复用）；过期（90 天）；schema 迁移与版本
- 会话安全：默认不自动锁（设置窗可改 5/15/30 分钟）；5 次失败锁定 30 分钟
- 审计：操作落库 + 解锁时清理 90 天前记录
- 导入导出：加密导出（.spb，独立导出密码）；明文导出命令已移除；导入去重统计
- 会话锁定：前端空闲计时 + 手动锁定，超时卸载界面并清空明文；锁定时长可设置并同步后端
- 备份：自包含加密备份包，可跨安装恢复
- 完整性：SQLite integrity_check + DB/配置 SHA-256 + 锁定基线比对
- Tauri 2：VaultRuntime + Mutex；命令；窗口/托盘；v2 配置 + 图标 + capabilities
- 启动器：Ctrl+Alt+Space 唤出、拼音首字母+子序列模糊搜索、回车复制并 30 秒清剪贴板
- 托盘：搜索/锁定/设置/退出；导入导出统一在设置窗
- 前端：独立设置窗（通用/导入导出/安全/审计/关于），不再打包 Element Plus
- 工程：AppError 统一错误（运行时边界已采用）；24 个单测/集成/安全测试通过；securepass bin 0 警告
- CI：`.github/workflows/ci.yml` 三平台矩阵（backend tests / frontend build / tauri build）

## 未完成
- 真机 / CI 实际运行验证（配置已就绪，需推送触发 GitHub Actions）
- 跨设备自动同步（当前用加密备份/恢复实现手工同步）

## 校验方式
- 后端：`cargo test --bin securepass_cli --no-default-features`
- 前端：`cd frontend && npm run build`
- 预览：`cd frontend && npm run dev`（index.html 为启动器，setting.html 为设置窗）
