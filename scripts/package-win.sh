#!/usr/bin/env bash
# 打包 Windows x64 便携版（在 Linux/WSL 上交叉编译）
#
# 用法：
#   ./scripts/package-win.sh                          # 普通便携版（依赖系统 WebView2）
#   ./scripts/package-win.sh --with-webview2          # 打包内置 WebView2（默认 ./.webview2-fixed）
#   ./scripts/package-win.sh --with-webview2 /path/to/固定版运行时
#
# --with-webview2 需要一个已解压的 WebView2 Fixed Version 运行时目录，
# 目录里必须包含 msedgewebview2.exe（微软官网 WebView2 → Fixed Version 下载后解压）。
# 打包后会额外生成 Hellopass.cmd，它设置 WEBVIEW2_BROWSER_EXECUTABLE_FOLDER
# 再启动程序，从而完全离线、不依赖系统安装的 WebView2 Runtime。
# 打包后会额外生成 <zip名>.sha256 校验文件（sha256sum 格式，供用户核对下载完整性）。
set -euo pipefail
cd "$(dirname "$0")/.."

WITH_WEBVIEW2=0
WV2_DIR=".webview2-fixed"
# 版本号取自 tauri.conf.json（打包产物实际使用的版本）
VERSION=$(python3 -c "import json;print(json.load(open('tauri.conf.json'))['version'])")

while [ $# -gt 0 ]; do
  case "$1" in
    --with-webview2)
      WITH_WEBVIEW2=1
      shift
      if [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; then
        WV2_DIR="$1"
        shift
      fi
      ;;
    -h|--help)
      sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "未知参数: $1（可用: --with-webview2 [目录]）" >&2
      exit 2
      ;;
  esac
done

if [ "$WITH_WEBVIEW2" = 1 ] && [ ! -f "$WV2_DIR/msedgewebview2.exe" ]; then
  echo "未找到 WebView2 固定版运行时: $WV2_DIR/msedgewebview2.exe" >&2
  echo "请从 https://developer.microsoft.com/microsoft-edge/webview2/ 下载 Fixed Version" >&2
  echo "解压后放到 $WV2_DIR，或用 --with-webview2 <目录> 指定路径。" >&2
  exit 1
fi

command -v cargo >/dev/null || { echo "缺少 cargo" >&2; exit 1; }
command -v cargo-xwin >/dev/null || {
  echo "缺少 cargo-xwin，请先运行: cargo install cargo-xwin --locked" >&2
  exit 1
}
command -v zip >/dev/null || { echo "缺少 zip" >&2; exit 1; }

rustup target list --installed | grep -qx x86_64-pc-windows-msvc || \
  rustup target add x86_64-pc-windows-msvc

echo "==> 编译 Windows x64 (release)"
cargo tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle

OUT=dist-win
rm -rf "$OUT"
mkdir -p "$OUT"
cp target/x86_64-pc-windows-msvc/release/hellopass.exe "$OUT/Hellopass.exe"
cp LICENSE "$OUT/LICENSE"
cp THIRD_PARTY_LICENSES.md "$OUT/THIRD_PARTY_LICENSES.md"

if [ "$WITH_WEBVIEW2" = 1 ]; then
  echo "==> 内置 WebView2 运行时: $WV2_DIR"
  mkdir -p "$OUT/webview2"
  cp -a "$WV2_DIR"/. "$OUT/webview2"/
  cat > "$OUT/Hellopass.cmd" <<'EOF'
@echo off
rem Use the bundled fixed-version WebView2 runtime instead of the system one.
set "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=%~dp0webview2"
start "" "%~dp0Hellopass.exe"
EOF
  sed -i 's/$/\r/' "$OUT/Hellopass.cmd"
fi

# 产物命名规则（两种分支保持一致），README 与打包收尾都引用它
if [ "$WITH_WEBVIEW2" = 1 ]; then
  ZIP="Hellopass-$VERSION-win64-webview2.zip"
else
  ZIP="Hellopass-$VERSION-win64.zip"
fi

cat > "$OUT/README.txt" <<EOF
Hellopass $VERSION  (Windows x64 便携版)
========================================

【运行】
EOF
if [ "$WITH_WEBVIEW2" = 1 ]; then
cat >> "$OUT/README.txt" <<'EOF'
双击 Hellopass.cmd（不是 exe），它会使用同目录 webview2\ 下的固定版运行时，
完全不依赖系统是否安装 WebView2。
直接双击 Hellopass.exe 也可以，但那样会回退到系统 WebView2。
EOF
else
cat >> "$OUT/README.txt" <<'EOF'
双击 Hellopass.exe。
程序常驻系统托盘，不会弹出主窗口。
运行需要系统已安装 WebView2 Runtime（Win11 及较新 Win10 已内置）。
EOF
fi
cat >> "$OUT/README.txt" <<EOF

1. 按 Ctrl+Alt+Space 唤出搜索窗，Ctrl+Shift+L 立即锁定（都可在托盘「设置…」里改键）
   输入即搜，↑↓ 选择，Enter 复制密码，Ctrl+Enter 复制用户名，
   → 查看详情，Ctrl+N 新建，Esc 收起
2. 托盘菜单：搜索 / 立即锁定 / 设置… / 退出；导入导出在设置窗完成
3. 首次启动会要求设置主密码：至少 12 位，含大小写字母、数字和特殊字符

【数据位置】
%APPDATA%\com.hellopass.app\hellopass.db
%APPDATA%\com.hellopass.app\config\master_password_config.json
%APPDATA%\com.hellopass.app\config\hotkey.json
内容以 AES-256-GCM 加密，主密钥由主密码经 PBKDF2-HMAC-SHA256（600,000 次）派生。

【许可证】
- 本项目采用 BSD-3-Clause，见 LICENSE
- 第三方组件许可见 THIRD_PARTY_LICENSES.md

【系统要求】
- Windows 10 1809 及以上 / Windows 11

【校验压缩包】
随包提供 $ZIP.sha256，内容为「SHA256 值 + 两个空格 + 文件名」，
与 $ZIP 放在同一目录即可验证下载是否完整、未被篡改：
- PowerShell（在压缩包所在目录）：
    Get-FileHash .\\$ZIP -Algorithm SHA256
- CMD（在压缩包所在目录）：
    certutil -hashfile $ZIP SHA256
把输出值与 $ZIP.sha256 中记录的哈希逐位比对，一致即通过。

【注意事项】
- 程序未做代码签名，首次运行 SmartScreen 可能提示"未知发布者"，
  点"更多信息 → 仍要运行"即可。
- 免安装版：删除 exe 不会删除 %APPDATA% 下的数据。彻底清理需删除
  %APPDATA%\com.hellopass.app 目录。
- 忘记主密码无法恢复数据；建议定期用"导出密码"生成 .spb 加密备份。
EOF

cd "$OUT"
rm -f ./*.zip ./*.zip.sha256
zip -q -r "$ZIP" .

# 先生成 zip，再计算校验值：.sha256 绝不能被打进 zip 内部
if command -v sha256sum >/dev/null 2>&1; then
  HASH=$(sha256sum "$ZIP" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  HASH=$(shasum -a 256 "$ZIP" | awk '{print $1}')
elif command -v openssl >/dev/null 2>&1; then
  HASH=$(openssl dgst -sha256 "$ZIP" | awk '{print $NF}')
else
  echo "错误: 未找到可用的 SHA256 工具，请安装 coreutils(sha256sum)、perl(shasum) 或 openssl 后重试。" >&2
  exit 1
fi
# 防御性校验：必须是 64 位小写十六进制，避免写入半截/错误内容
case "$HASH" in
  *[!0-9a-f]*|"") echo "错误: SHA256 计算结果异常（'$HASH'），已中止。" >&2; exit 1 ;;
esac
if [ "${#HASH}" -ne 64 ]; then
  echo "错误: SHA256 长度异常（${#HASH} != 64），已中止。" >&2
  exit 1
fi
# 单行、相对当前目录的纯文件名，两个空格分隔，sha256sum -c 兼容
printf '%s  %s\n' "$HASH" "$ZIP" > "$ZIP.sha256"

echo
echo "==> 产物: $(pwd)"
ls -lh
echo
echo "==> SHA256 校验文件: $ZIP.sha256"
echo "$HASH  $ZIP"
echo "（上面这行可直接粘贴到 GitHub Release Notes）"
