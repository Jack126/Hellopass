#!/usr/bin/env bash
# 打包 Linux 便携版（tar.gz），可选同时生成 .deb
#
# 用法：
#   ./scripts/package-linux.sh            # 便携版 tar.gz（含 .desktop 与图标）
#   ./scripts/package-linux.sh --deb      # 便携版 + .deb 安装包（走 tauri bundler）
#
# 说明：
#   - 只打当前主机架构的包（x86_64 / aarch64），不做交叉编译。
#   - 便携版解压即用：./hellopass；也可按包内 README.txt 加入桌面菜单。
#   - 产物在 dist-linux/，每个压缩包/安装包都附 .sha256 校验文件。
set -euo pipefail
cd "$(dirname "$0")/.."

BUILD_DEB=0
# 版本号取自 tauri.conf.json（打包产物实际使用的版本）
VERSION=$(python3 -c "import json;print(json.load(open('tauri.conf.json'))['version'])")
ARCH=$(uname -m)

while [ $# -gt 0 ]; do
  case "$1" in
    --deb) BUILD_DEB=1; shift ;;
    -h|--help)
      sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "未知参数: $1（可用: --deb）" >&2
      exit 2
      ;;
  esac
done

case "$ARCH" in
  x86_64|amd64) ARCH=x86_64 ;;
  aarch64|arm64) ARCH=aarch64 ;;
  *) echo "未验证的架构: $ARCH，继续按当前架构打包" >&2 ;;
esac

command -v cargo >/dev/null || { echo "缺少 cargo" >&2; exit 1; }
command -v cargo-tauri >/dev/null || {
  echo "缺少 Tauri CLI，请先运行: cargo install tauri-cli --version ^2.0.0" >&2
  exit 1
}
command -v tar >/dev/null || { echo "缺少 tar" >&2; exit 1; }
if [ "$BUILD_DEB" = 1 ]; then
  command -v dpkg-deb >/dev/null || { echo "缺少 dpkg-deb，无法生成 .deb" >&2; exit 1; }
fi

if [ "$BUILD_DEB" = 1 ]; then
  echo "==> 编译 Linux $ARCH (release) 并生成 deb"
  cargo tauri build --bundles deb
else
  echo "==> 编译 Linux $ARCH (release)"
  cargo tauri build --no-bundle
fi

BIN="target/release/hellopass"
[ -x "$BIN" ] || { echo "未找到编译产物: $BIN" >&2; exit 1; }

OUT=dist-linux
rm -rf "$OUT"
mkdir -p "$OUT"

cp "$BIN" "$OUT/hellopass"
chmod 755 "$OUT/hellopass"
cp LICENSE "$OUT/LICENSE"
cp THIRD_PARTY_LICENSES.md "$OUT/THIRD_PARTY_LICENSES.md"
cp icons/128x128.png "$OUT/hellopass.png"

cat > "$OUT/Hellopass.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Hellopass
Comment=Secure local password manager
Exec=hellopass
Icon=hellopass
Terminal=false
Categories=Utility;Security;
Keywords=password;vault;security;
StartupNotify=false
EOF

TARNAME="Hellopass-$VERSION-linux-$ARCH.tar.gz"
# 只取与当前版本号匹配的 deb，避免 target 里残留的旧版本被打进产物
DEB_SRC=$(ls target/release/bundle/deb/Hellopass_${VERSION}_*.deb 2>/dev/null | head -n 1 || true)
DEB_NAME=""
if [ "$BUILD_DEB" = 1 ]; then
  if [ -n "$DEB_SRC" ]; then
    DEB_NAME=$(basename "$DEB_SRC")
  else
    echo "警告: 未在 target/release/bundle/deb 找到 .deb，仅生成便携版" >&2
  fi
fi

cat > "$OUT/README.txt" <<EOF
Hellopass $VERSION  (Linux $ARCH 便携版)
========================================

【运行】
解压后直接执行同目录的 ./hellopass。
程序常驻系统托盘，不弹出主窗口。

1. 按 Ctrl+Alt+Space 唤出搜索窗，Ctrl+Shift+L 立即锁定（都可在托盘「设置…」里改键）
   输入即搜，↑↓ 选择，Enter 复制密码，Ctrl+Enter 复制用户名，
   → 查看详情，Ctrl+N 新建，Esc 收起
2. 托盘菜单：搜索 / 立即锁定 / 设置… / 退出；导入导出在设置窗完成
3. 首次启动会要求设置主密码：至少 12 位，含大小写字母、数字和特殊字符

【加入应用菜单（可选，仅当前用户）】
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/128x128/apps
install -m755 hellopass ~/.local/bin/
install -m644 hellopass.png ~/.local/share/icons/hicolor/128x128/apps/
install -m644 Hellopass.desktop ~/.local/share/applications/
# ~/.local/bin 需在 PATH 中（多数发行版默认已加入，重新登录后生效）

【数据位置】
默认 ~/.local/share/com.hellopass.app/hellopass.db
（设置了 XDG_DATA_HOME 时为 \$XDG_DATA_HOME/com.hellopass.app/hellopass.db）
同目录 config/ 下为主密码配置、热键与界面偏好。
内容以 AES-256-GCM 加密，主密钥由主密码经 PBKDF2-HMAC-SHA256（600,000 次）派生。

【系统要求】
- WebKitGTK 4.1 与 GTK 3 运行库：
  Debian/Ubuntu: sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0
  Fedora:        sudo dnf install webkit2gtk4.1 gtk3
- 系统托盘需要 AppIndicator 支持（多数桌面环境已自带）

【许可证】
- 本项目采用 BSD-3-Clause，见 LICENSE
- 第三方组件许可见 THIRD_PARTY_LICENSES.md

【校验压缩包】
sha256sum -c $TARNAME.sha256
输出 "OK" 即文件完整、未被篡改。

【注意事项】
- 程序未做代码签名，首次运行部分桌面环境可能提示来源未知。
- 免安装版：删除二进制不会删除 ~/.local/share/com.hellopass.app 下的数据。
- 忘记主密码无法恢复数据；建议定期用「导出密码」生成 .spb 加密备份。
EOF

if [ -n "$DEB_NAME" ]; then
cat >> "$OUT/README.txt" <<EOF

【.deb 安装包】
$DEB_NAME 可直接安装（Debian/Ubuntu 系）：
  sudo apt install ./$DEB_NAME
卸载：sudo apt remove hellopass
EOF
fi

# 先打包，再把 .sha256 写在外面，避免校验文件被打进压缩包
TAR="$OUT/$TARNAME"
TMP_TAR=$(mktemp)
tar -czf "$TMP_TAR" -C "$OUT" .
mv "$TMP_TAR" "$TAR"
chmod 644 "$TAR"

# .deb 在打 tar 之后再放进来，避免把安装包塞进便携压缩包
if [ -n "$DEB_NAME" ]; then
  cp "$DEB_SRC" "$OUT/$DEB_NAME"
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  else
    echo "错误: 未找到可用的 SHA256 工具（sha256sum / shasum / openssl）" >&2
    return 1
  fi
}

write_sha256() {
  local file="$1"
  local name hash
  name=$(basename "$file")
  hash=$(sha256_of "$file") || exit 1
  case "$hash" in
    *[!0-9a-f]*|"") echo "错误: $name 的 SHA256 计算结果异常（'$hash'），已中止。" >&2; exit 1 ;;
  esac
  if [ "${#hash}" -ne 64 ]; then
    echo "错误: $name 的 SHA256 长度异常（${#hash} != 64），已中止。" >&2
    exit 1
  fi
  printf '%s  %s\n' "$hash" "$name" > "$file.sha256"
  echo "$hash  $name"
}

cd "$OUT"
rm -f ./*.tar.gz.sha256 ./*.deb.sha256
echo
echo "==> 生成校验文件"
write_sha256 "$TARNAME"
if [ -n "$DEB_NAME" ]; then
  write_sha256 "$DEB_NAME"
fi

echo
echo "==> 产物: $(pwd)"
ls -lh
echo
echo "（上面的 “哈希  文件名” 可直接粘贴到 GitHub Release Notes）"
