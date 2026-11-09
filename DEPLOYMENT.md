# SecurePass 部署指南

SecurePass 是基于 Tauri 2 的跨平台**桌面**密码管理器。打包与分发使用 Tauri 官方流程，早期仓库中的多 bin / Docker / Web 部署方式已废弃删除。

## 一、开发运行

```bash
# 1. 安装 Tauri CLI（v2）
cargo install tauri-cli --version ^2.0.0

# 2. 安装前端依赖并启动开发
cd frontend && npm install && cd ..
cargo tauri dev
```

> Linux 需系统依赖：webkit2gtk-4.1、libdbus-1-dev、librsvg、build-essential 等（详见 Tauri 2 官方文档）。
> 无 GUI 的无头环境可用 `cargo test --bin securepass --no-default-features` 运行核心单元测试。

## 二、构建可分发包

```bash
cargo tauri build
```

按当前平台产出安装包（Windows: .msi/.exe；macOS: .app/.dmg；Linux: .deb/.rpm/AppImage）。
安装包相关配置在 `tauri.conf.json` 的 `bundle` 段（图标、分类等），应用标识符在顶层 `identifier`。

## 三、数据存放

- 数据库与配置位于各平台的应用数据目录（由 `app.path().app_data_dir()` 解析）。
- 仓库根目录遗留的 `securepass.db` / `backup.db` 为本地运行产物，建议加入 `.gitignore`，不要提交。

## 四、已移除的旧部署方式（仅供参考）

以下文件已不再适用并从仓库删除：
- `scripts/build-{linux,macos,windows,freebsd}.sh`、`scripts/build-all.sh`（针对不存在的 `securepass_complete` CLI）
- `build/` 目录（AppImage / 便携式打包脚本与旧二进制）
- `Dockerfile`、`Dockerfile.simple`、`docker-compose.yml`、`nginx.conf`、`monitoring/`（Web 容器化部署）
- `Cargo_simple.toml`、`Cargo_full.toml`（历史变体清单）
- `DEPLOYMENT-GUIDE.md` 中的 CLI 子命令（add/list/search/backup/restore）用法

Tauri 2 的权限由 `capabilities/` 下的能力文件控制（当前为 `capabilities/default.json`，含 `core:default`）；图标位于 `icons/`，打包资源位于 `assets/`。
