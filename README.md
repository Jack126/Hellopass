# SecurePass 密码管理器

跨平台密码管理器，采用 Vue + Rust + Tauri 架构，注重安全性，支持企业内部使用。

## 📁 项目结构

```
passwd/
├── src/                          # Rust 后端
│   ├── main.rs                   # Tauri 主入口
│   ├── security/                 # 安全模块
│   │   ├── mod.rs
│   │   ├── crypto.rs             # 加密算法
│   │   ├── master_password.rs   # 主密码管理
│   │   └── policies.rs           # 安全策略
│   ├── database/                 # 数据库模块
│   │   ├── mod.rs
│   │   ├── models.rs             # 数据模型
│   │   └── schema.sql            # 数据库结构
│   ├── api/                      # API 接口
│   │   ├── mod.rs
│   │   ├── commands.rs           # 命令处理
│   │   ├── import_export.rs       # 导入导出
│   │   └── auth.rs               # 认证模块
│   └── utils/                    # 工具函数
├── frontend/                     # Vue 前端
│   ├── src/
│   │   ├── components/           # Vue 组件
│   │   │   ├── PasswordList.vue
│   │   │   ├── PasswordCard.vue
│   │   │   ├── AddPasswordDialog.vue
│   │   │   ├── ExportDialog.vue
│   │   │   └── SettingsDialog.vue
│   │   ├── stores/               # Pinia 状态管理
│   │   ├── utils/                # 工具函数
│   │   └── types/                # TypeScript 类型
│   ├── public/
│   └── package.json
├── tauri.conf.json               # Tauri 配置
├── Cargo.toml                   # Rust 依赖
├── todo.md                       # 开发任务列表
├── README.md                     # 项目说明
└── security-architecture.md      # 安全架构设计
```

## 🎯 核心功能

### ✅ 已完成
- [x] 功能逻辑点梳理
- [x] 安全架构设计
- [x] 加密算法选择（AES-256-GCM + PBKDF2-HMAC-SHA256）
- [x] 数据库架构设计
- [x] API接口设计
- [x] 主密码系统设计
- [x] 导入导出机制设计

### 🔄 开发中
- [ ] Rust 后端核心功能实现
- [ ] Vue 前端界面开发
- [ ] Tauri 应用集成
- [ ] 安全测试和优化

### ⏳ 待开发

#### 1. 核心模块开发
- [ ] **Rust 后端**
  - [ ] 加密模块实现 (`src/security/crypto.rs`)
  - [ ] 主密码管理 (`src/security/master_password.rs`)
  - [ ] 数据库操作 (`src/database/`)
  - [ ] API 接口 (`src/api/`)
  - [ ] 错误处理和日志

- [ ] **Vue 前端**
  - [ ] 手动输入密码界面 (`AddPasswordDialog.vue`)
  - [ ] 多选导出界面 (`ExportDialog.vue`)
  - [ ] 密码列表显示 (`PasswordList.vue`)
  - [ ] 密码强度检测
  - [ ] 搜索和过滤功能

- [ ] **Tauri 集成**
  - [ ] 配置文件 (`tauri.conf.json`)
  - [ ] 跨平台打包
  - [ ] 系统集成（文件关联、快捷方式）

#### 2. 安全功能
- [ ] **完整性校验**
  - [ ] 文件完整性检查
  - [ ] 运行时完整性验证
  - [ ] 防篡改机制

- [ ] **审计日志**
  - [ ] 操作记录
  - [ ] 安全事件检测
  - [ ] 异常行为告警

- [ ] **密码策略**
  - [ ] 密码强度验证
  - [ ] 密码过期提醒
  - [ ] 密码历史记录

#### 3. 导入导出功能
- [ ] **CSV 导入**
  - [ ] 格式验证
  - [ ] 批量导入
  - [ ] 错误处理

- [ ] **加密导出**
  - [ ] 多选导出
  - [ ] 导出文件格式
  - [ ] 导入验证

- [ ] **数据迁移**
  - [ ] 老版本数据迁移
  - [ ] 跨设备同步
  - [ ] 备份恢复

## 🔐 安全特性

### 加密算法
- **主加密**: AES-256-GCM
- **密钥派生**: PBKDF2-HMAC-SHA256 (100,000 次迭代)
- **哈希算法**: SHA-256
- **密钥长度**: 256 位 (32 字节)

### 数据存储
- **明文字段**: 网站名称、URL、用户名
- **加密字段**: 密码、备注
- **数据库**: SQLite
- **文件权限**: 600 (仅所有者可读写)

### 安全策略
- **主密码**: 最小 12 位，必须包含大小写字母、数字、特殊字符
- **会话管理**: 30 分钟超时，5 次错误尝试锁定
- **审计日志**: 记录所有操作，保留 90 天
- **备份加密**: 所有备份数据加密存储

## 🚀 快速开始

### 环境要求
- **Rust**: 1.70+
- **Node.js**: 18+
- **Vue**: 3+
- **Tauri**: 1.5+

### 安装依赖
```bash
# Rust 依赖
cargo install tauri-cli

# Node.js 依赖
cd frontend
npm install
cd ..

# Tauri 依赖
cargo install tauri-cli --version 1.5
```

### 开发模式
```bash
# 启动前端开发服务器
cd frontend
npm run dev

# 启动 Tauri 开发服务器
cd ..
cargo tauri dev
```

### 构建应用
```bash
# 构建所有平台
cargo tauri build

# 构建特定平台
cargo tauri build --target x86_64-pc-windows-msvc
cargo tauri build --target x86_64-apple-darwin
cargo tauri build --target x86_64-unknown-linux-gnu
```

## 📋 开发任务

详细的开发任务列表请参考 `todo.md` 文件。

## 📝 文档

- [安全架构设计](security-architecture.md)
- [开发任务列表](todo.md)
- [API 接口文档](docs/api.md)
- [部署指南](docs/deployment.md)

## 📞 联系方式

如有问题或建议，请通过以下方式联系：
- 项目仓库: [GitHub Repository]
- 问题反馈: [Issues]
- 开发讨论: [Discussions]

---

*最后更新: 2024-01-15*
*维护者: 开发团队*