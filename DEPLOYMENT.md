# SecurePass 部署指南

## 🚀 部署概述

SecurePass 是一个基于 Vue + Rust + Tauri 的桌面密码管理器应用，支持多种部署方式。

## 📋 系统要求

### 开发环境
- **Rust**: 1.70+
- **Node.js**: 18+
- **npm**: 8+
- **Tauri CLI**: 1.5+
- **操作系统**: Windows 10+, macOS 10.14+, Ubuntu 18.04+

### 生产环境
- **操作系统**: Windows 10+, macOS 10.14+, Ubuntu 18.04+
- **内存**: 至少 512MB RAM
- **存储**: 至少 100MB 可用空间
- **网络**: 可选（用于备份同步）

## 🛠️ 开发环境搭建

### 1. 克隆项目
```bash
git clone https://github.com/your-username/securepass.git
cd securepass
```

### 2. 安 Rust 后端
```bash
# 安装 Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 Tauri 工具链
cargo install tauri-cli

# 构建后端
cargo build --release
```

### 3. 安前端依赖
```bash
cd frontend
npm install
cd ..
```

### 4. 开发模式运行
```bash
# 后端开发模式
cargo run --bin securepass_complete

# 前端开发模式
cd frontend
npm run dev
```

## 🏗️ 构建和打包

### Web 版本构建
```bash
cd frontend
npm run build
```

### 桌面应用构建
```bash
# 构建所有平台
cd frontend
npm run tauri:build

# 构建特定平台
npm run tauri:build:win   # Windows
npm run tauri:build:mac   # macOS
npm run tauri:build:linux # Linux
```

### 构建产物
- **Web 版本**: `frontend/dist/`
- **桌面应用**: `frontend/src-tauri/target/release/`

## 📦 部署选项

### 1. Web 部署
```bash
# 部署到静态网站托管
cd frontend/dist
# 使用 GitHub Pages, Netlify, Vercel 等平台部署
```

### 2. 桌面应用部署
```bash
# Windows
cd frontend/src-tauri/target/release
# 创建安装程序
.\securepass.exe

# macOS
cd frontend/src-tauri/target/release
# 创建 DMG
sudo create-dmg --volname "SecurePass" --volicon "../icons.icns" --window-pos 200 120 --window-size 600 300 --icon-size 100 --icon "securepass.app" 175 120 --hide-extension "securepass.app" securepass.dmg

# Linux
cd frontend/src-tauri/target/release
# 创建 DEB 包
dpkg-deb -b securepass.deb
```

### 3. 容器化部署
```bash
# 构建容器
docker build -t securepass .

# 运行容器
docker run -d -p 8080:8080 securepass
```

## 🔧 配置文件

### Tauri 配置
```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:3000",
    "distDir": "../dist"
  },
  "package": {
    "productName": "SecurePass",
    "version": "1.0.0"
  },
  "tauri": {
    "bundle": {
      "active": true,
      "category": "Productivity",
      "copyright": "Copyright © 2024 SecurePass Team",
      "deb": {
        "depends": ["libgtk-3-0", "libayatana-appindicator3-1", "libwebkit2gtk-4.0-37"]
      },
      "externalBin": [],
      "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png"],
      "identifier": "com.securepass.app",
      "longDescription": "A secure password manager with encryption and synchronization.",
      "macOS": {
        "entitlements": "entitlements.plist",
        "exceptionDomain": "",
        "providerShortName": "SecurePass",
        "signingIdentity": "Developer ID Application: SecurePass Team"
      },
      "resources": [],
      "shortDescription": "SecurePass - Secure Password Manager",
      "targets": "all",
      "windows": {
        "certificateThumbprint": "",
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    },
    "security": {
      "csp": "default-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'"
    },
    "updater": {
      "active": false,
      "dialog": true,
      "pubkey": "1DNi7yQdOQD4A6M7MqJ3fF7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7yZ7