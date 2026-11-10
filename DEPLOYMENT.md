# Hellopass 部署指南

Hellopass 是基于 Tauri 2 的跨平台**桌面**密码管理器。打包与分发使用 Tauri 官方流程，早期仓库中的多 bin / Docker / Web 部署方式已废弃删除。

## 一、开发运行

```bash
# 1. 安装 Tauri CLI（v2）
cargo install tauri-cli --version ^2.0.0

# 2. 安装前端依赖并启动开发
cd frontend && npm install && cd ..
cargo tauri dev
```

> Linux 需系统依赖：webkit2gtk-4.1、libdbus-1-dev、librsvg、build-essential 等（详见 Tauri 2 官方文档）。
> 无 GUI 的无头环境可用 `cargo test --bin hellopass_cli --no-default-features` 运行核心单元测试。

## 二、构建可分发包

### 2.1 当前平台

```bash
cargo tauri build
```

按当前平台产出安装包（Windows: .msi/.exe；macOS: .app/.dmg；Linux: .deb/.rpm/AppImage）。
安装包相关配置在 `tauri.conf.json` 的 `bundle` 段（图标、分类等），应用标识符在顶层 `identifier`。

### 2.2 各平台实测结果（2026-11-09，Linux/WSL 主机）

| 平台 | 命令 | 结果 |
|---|---|---|
| Linux | `cargo tauri build --bundles deb` | ✅ 产出 `Hellopass_1.0.0_amd64.deb`（6.1 MB） |
| Linux | `cargo tauri build --bundles appimage` | ❌ 需从 GitHub 下载 AppRun，本机 DNS 不可达 |
| Linux | `cargo tauri build --bundles rpm` | ❌ 需要 `rpmbuild`（未安装） |
| Windows x64 | `./scripts/package-win.sh` | ✅ 交叉编译产出便携 zip（cargo-xwin + clang-cl/lld-link） |
| Windows x64 | `cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` | ✅ 编译通过；NSIS 安装包需从 GitHub 下载工具，本机不可达 |
| macOS | 只能在 macOS 主机上 `cargo tauri build` | ⛔ Linux 无法交叉（需 Apple SDK、签名与公证） |
| FreeBSD | — | ⛔ Tauri 2 官方不支持 |

### 2.3 Windows 打包与 WebView2 选项

```bash
./scripts/package-win.sh                      # 便携版：依赖系统安装的 WebView2 Runtime
./scripts/package-win.sh --with-webview2      # 内置固定版 WebView2（默认读 ./.webview2-fixed）
./scripts/package-win.sh --with-webview2 DIR  # 指定固定版运行时目录
```

- 固定版运行时从微软 WebView2 页面（Fixed Version）下载并解压，目录内应包含 `msedgewebview2.exe`。
- 内置模式会额外生成 `Hellopass.cmd`，它设置 `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` 指向同目录 `webview2/`，
  双击该 cmd 启动即可完全离线，不依赖系统是否装过 WebView2。
- 体积参考：exe 约 12 MB；固定版运行时约 150–200 MB，压缩后 zip 约 60–80 MB。

### 2.4 macOS / FreeBSD

- **macOS**：必须在 macOS 主机上构建。`cargo tauri build` 产出 `.app` / `.dmg`；分发需要开发者证书做签名与公证。
- **FreeBSD**：Tauri 2 的 `wry`/`tao` 仅支持 linux/windows/macos/android/ios，源码里按 `target_os = "linux"` 分支处理，
  FreeBSD 不在支持范围，官方无构建路径；如需支持需自行移植 WebView 绑定。

## 三、数据存放

- 数据库与配置位于各平台的应用数据目录（由 `app.path().app_data_dir()` 解析）。
- 仓库根目录遗留的 `hellopass.db` / `backup.db` 为本地运行产物，建议加入 `.gitignore`，不要提交。

## 四、已移除的旧部署方式（仅供参考）

以下文件已不再适用并从仓库删除：
- `scripts/build-{linux,macos,windows,freebsd}.sh`、`scripts/build-all.sh`（针对不存在的 `hellopass_complete` CLI）
- `build/` 目录（AppImage / 便携式打包脚本与旧二进制）
- `Dockerfile`、`Dockerfile.simple`、`docker-compose.yml`、`nginx.conf`、`monitoring/`（Web 容器化部署）
- `Cargo_simple.toml`、`Cargo_full.toml`（历史变体清单）
- `DEPLOYMENT-GUIDE.md` 中的 CLI 子命令（add/list/search/backup/restore）用法

Tauri 2 的权限由 `capabilities/` 下的能力文件控制（当前为 `capabilities/default.json`，含 `core:default`）；图标位于 `icons/`，打包资源位于 `assets/`。
