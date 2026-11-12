# Hellopass 密码管理器 — 补充文档

本文件收录主 README 之外的详细内容：项目结构、已实现功能、快速开始（构建与测试）、相关文档索引。

快速使用、安全参数、截图与未完成项请见 [README.md](README.md)。

## 📁 项目结构

```
passwd/
├── src/                          # Rust 后端
│   ├── main.rs                   # hellopass_cli 自测入口（无 GUI 依赖）
│   ├── main_tauri.rs             # hellopass Tauri 2 GUI 入口
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
│       ├── identity.rs           # 导入身份标识（名称+网址+用户名）
│       ├── log.rs                # 统一日志
│       ├── pinyin.rs             # 拼音首字母
│       └── window_size.rs        # 启动器窗口尺寸计算
├── frontend/                     # Vue 3（无 UI 框架）
│   └── src/
│       ├── launcher/             # 热键搜索弹窗：Launcher.vue / fuzzy.ts / ipc.ts
│       ├── settings/             # 托盘打开的设置窗：SettingsApp.vue
│       └── shared/ipc.ts         # 不依赖 UI 库的 invoke 封装
├── icons/                        # 应用/托盘图标（png/ico/icns）
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

#### 导入规则

三种入口：

| 入口 | 文件 | 需要的密码 | 说明 |
| --- | --- | --- | --- |
| 设置 → 导入密码 | `.spb` | 导出密码 | 单条密码加密包，AES-256-GCM |
| 设置 → 导入密码 | `.json` / `.csv` | 无 | 明文导入，落库前用本机主密钥重新加密 |
| 设置 → 备份恢复 | `.spbbackup` | 备份对应的主密码 | 自包含加密备份 |

**CSV 列名**（首行表头，忽略大小写与首尾空白）：

| 字段 | 可用列名 | 必填 |
| --- | --- | --- |
| 站点名 | `site_name` / `name` / `title` | 是（缺失或为空会报错） |
| 网址 | `site_url` / `url` | 否 |
| 用户名 | `username` | 否（浏览器导出里可能为空） |
| 密码 | `password` | 是（列必须存在；空值行会被跳过） |
| 备注 | `notes` / `note` | 否 |
| 标签 | `tags` | 否（分号分隔） |

**CSV 解析**：支持 UTF-8 BOM、CRLF、引号包裹的逗号/换行、`""` 转义；浏览器（Chrome/Edge）导出的
`name,url,username,password,note` 可直接导入。

**身份与覆盖规则**：

- 身份标识为「名称 + 网址 + 用户名」三项，全部相同才视为同一条目，因此同一站点可保留多个账号。
- 归一化：名称、用户名忽略首尾空白与大小写；网址忽略协议、`www.`、末尾斜杠与大小写，
  端口、路径、参数保留（`/a` 与 `/b` 视为不同条目）。
- 命中已有条目时覆盖：站点名、网址、用户名、密码、备注、标签、优先级用导入值替换，
  `created_at` 保留、`updated_at` 刷新；导入未提供的备注/标签/优先级沿用旧值。
- 空密码行直接跳过，避免把已有密码覆盖为空。
- 覆盖导入不套用「最近 5 次密码不可复用」限制（导入数据视为权威来源），但仍写入密码历史。

**上限与失败处理**：

- 单个文件 ≤ **5 MB**（三种导入入口都在解析前检查；前端读文件前也会先看 `File.size`）。
- 单次 ≤ **5000 条**，超限整批拒绝，不写入任何条目。
- CSV 缺少名称列/密码列、某行缺少站点名时整批报错（fail closed）。
- 导入结果返回「新增 / 覆盖 / 跳过 / 失败」四项统计，跳过与失败原因写入 `errors`。

**安全**：导入前必须已解锁；三种入口落库前都用本机主密钥 AES-256-GCM 重新加密，明文不落盘；
`.spb` / `.spbbackup` 校验版本、salt 长度并做 GCM 认证解密，密码错误或文件被改直接拒绝；
SQL 全部参数绑定。

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
- `VaultRuntime` 以 `Mutex` 注入，暴露 24 个 command
- 程序化创建主窗口 + 系统托盘
- 配置为 Tauri 2 结构，含图标与 capabilities

### 工程与测试
- 24 个单元 / 集成 / 安全测试（密文篡改、错误密钥、错误主密码、错误备份密码）
- 统一错误类型 `AppError`（运行时边界已采用，错误统一记日志）
- GitHub Actions 三平台 CI（后端测试 / 前端构建 / Tauri 打包）

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- Node.js 18+
- Tauri 2 CLI（真机开发/打包）
- Linux 需 `webkit2gtk-4.1` 与 `libdbus-1-dev`

### 无头核心测试（无需 GUI 依赖）
```bash
cargo test --bin hellopass_cli --no-default-features
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
最后更新：2026-11-12
