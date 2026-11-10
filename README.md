# SecurePass 密码管理器

跨平台密码管理器，采用 Vue 3 + Rust + Tauri 2 架构；密码使用 AES-256-GCM 加密，主密钥由 PBKDF2-HMAC-SHA256（600,000 次迭代）从主密码派生。

## 🚀 使用方式

- 启动后常驻托盘，不显示主窗口。
- 按 `Ctrl+Alt+Space` 唤出搜索窗，`Ctrl+Shift+L` 立即锁定（两个热键都可在设置窗里自由改键，录制期间会临时注销）：输入即搜（支持拼音首字母，如 `wx` → 微信），`↑↓` 选择，`Enter` 复制密码并收起（30 秒后自动清剪贴板），`Ctrl+Enter` 复制用户名，`→` 查看详情，`Ctrl+N` 新建。
- 托盘菜单：搜索 / 立即锁定 / 设置… / 退出；导入导出统一在设置窗里完成。
- 设置窗包含窗口大小（自动/紧凑/标准/大）、结果条数（自动填满/固定 N 条）、自动锁定时长、修改主密码、导入导出、备份恢复、完整性检查与审计日志。

## 📁 项目结构

```
passwd/
├── src/                          # Rust 后端
│   ├── main.rs                   # securepass_cli 自测入口（无 GUI 依赖）
│   ├── main_tauri.rs             # securepass Tauri 2 GUI 入口
│   ├── security/
│   │   ├── crypto.rs             # AES-256-GCM + PBKDF2
│   │   ├── master_password.rs    # 主密码设置/验证/salt 持久化
│   │   ├── key_management.rs     # 密钥管理
│   │   └── policies.rs           # 密码强度评估
│   ├── database/
│   │   ├── mod.rs                # 数据库初始化/建表/统计
│   │   ├── operations.rs         # 加解密 CRUD、审计、历史、过期
│   │   └── schema.sql            # 建表脚本（文档用途）
│   ├── api/
│   │   ├── dto.rs                # 请求/响应 DTO
│   │   ├── runtime.rs            # VaultRuntime：会话安全/CRUD/导入导出/审计
│   │   └── mod.rs
│   └── utils/
│       ├── app_state.rs          # CLI 用的简单状态
│       └── log.rs                # 统一日志
├── frontend/                     # Vue 3（无 UI 框架）
│   └── src/
│       ├── launcher/             # 热键搜索弹窗：Launcher.vue / fuzzy.ts / ipc.ts
│       ├── settings/             # 托盘打开的设置窗：SettingsApp.vue
│       └── shared/ipc.ts         # 不依赖 UI 库的 invoke 封装
├── icons/                        # 应用/托盘图标（png/ico/icns）
├── assets/                       # 打包资源
├── capabilities/default.json     # Tauri 2 权限
├── tauri.conf.json               # Tauri 2 配置
├── Cargo.toml
├── todo.md
└── README.md
```

## ✅ 已实现功能

### 加密与主密码
- AES-256-GCM 认证加密（防篡改）
- PBKDF2-HMAC-SHA256，600,000 次迭代
- salt 持久化 + 验证令牌校验主密码
- 修改主密码时用旧密钥解密、新密钥重加密（rekey）

### 数据库与 CRUD
- SQLite（rusqlite），落库前加密、读取时解密
- 增/查全部/按 id 查/模糊搜索/更新/删除
- 数据库文件权限 0600

### 会话安全
- 默认不自动锁定（启动器形态），可在设置中改为 5/15/30 分钟
- 手动锁定或退出时清空主密钥与界面明文；剪贴板里若仍是刚复制的密码也会立即清空
- 5 次错误尝试锁定 30 分钟
- 解锁/锁定/失败均写入审计日志

### 审计日志
- `audit_logs` 记录 unlock/lock/add/update/delete/change_master_password
- 提供 `get_audit_logs` 查询（最近 200 条）

### 密码策略
- 强度评估（长度、大小写、数字、特殊字符、常见弱口令）
- 过期提醒：默认最长 90 天，提供到期清单
- 历史记录：记住最近 5 次密码，禁止复用

### 导入导出
- 加密导出：勾选要导出的条目，用独立「导出密码」派生密钥，输出自包含 .spb 包（AES-256-GCM）
- 明文 JSON/CSV 导出命令已移除，避免一键导出全部明文
- 导入支持 .spb 加密包与 JSON/CSV，按 站点名+用户名 去重，返回统计

### 备份与完整性
- 加密备份：自包含备份包（内嵌 salt/iterations），可跨安装用同一主密码恢复
- 完整性检查：SQLite integrity_check + 数据库/配置文件 SHA-256（安全概览可触发）
- 运行时基线：正常锁定写入 DB 文件哈希，下次解锁比对并告警
- 数据迁移：`schema_meta` 版本记录 + 启动时补齐旧库缺失列
- 密码生成器：`crypto.getRandomValues` 生成 16 位强随机密码

### 前端
- 主密码设置/解锁弹窗
- 密码列表、新增/编辑/删除、搜索、分类
- 导入（读文件）/ 加密导出（后端写入系统下载目录并提示完整路径）
- 空闲自动锁定（默认 30 秒，设置范围 10 秒 ~ 5 分钟）与手动锁定按钮

### Tauri 2
- `VaultRuntime` 以 `Mutex` 注入，暴露 24 个命令
- 程序化创建主窗口 + 系统托盘
- 配置为 Tauri 2 结构，含图标与 capabilities

### 工程与测试
- 24 个单元 / 集成 / 安全测试（密文篡改、错误密钥、错误主密码、错误备份密码）
- 统一错误类型 `AppError`（运行时边界已采用，错误统一记日志）
- GitHub Actions 三平台 CI（后端测试 / 前端构建 / Tauri 打包）

## ⏳ 尚未完成（按处理顺序）
1. 真机 / CI 实际运行验证（`github/workflows/ci.yml` 已配置三平台矩阵，需推送触发）
2. 跨设备自动同步（当前用加密备份/恢复实现手工迁移）
3. 更大规模的模糊测试 / 渗透测试

## 🔐 安全参数
- 主加密：AES-256-GCM
- 密钥派生：PBKDF2-HMAC-SHA256（600,000 次）
- 密钥长度：256 位
- 明文存储：网站名称、URL、用户名；密码与备注加密存储
- 文件权限：600
- 主密码：最少 12 位，含大小写、数字、特殊字符
- 会话：30 秒空闲超时（最大 5 分钟），5 次失败锁定 30 分钟
- 密码历史：5 条（主密钥加密存储）；过期：90 天
- 敏感内存：主密钥与解密缓冲区用 `zeroize` 在释放时清零

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- Node.js 18+
- Tauri 2 CLI（真机开发/打包）
- Linux 需 `webkit2gtk-4.1` 与 `libdbus-1-dev`

### 无头核心测试（无需 GUI 依赖）
```bash
cargo test --bin securepass_cli --no-default-features
```

### 前端构建与单元测试
```bash
cd frontend
npm install
npm test        # 锁定清理 / clearSensitiveState 静态守护
npm run build
```

### 桌面开发（需 GUI 依赖）
```bash
npm install --prefix frontend
cargo install tauri-cli --version ^2.0.0
cargo tauri dev
```

### 打包
```bash
cargo tauri build
```

## 📝 文档
- [安全架构设计](security-architecture.md)
- [部署指南](DEPLOYMENT.md)
- [项目总结](PROJECT_SUMMARY.md)
- [开发任务列表](todo.md)

---
最后更新：2026-10-20
