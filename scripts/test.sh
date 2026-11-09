#!/bin/bash
# SecurePass 验证脚本
# 无头环境运行核心 Rust 单元测试（不依赖 GUI）
# 桌面 GUI 验证需在带 webkit2gtk-4.1 / libdbus-1-dev 的环境执行: cargo tauri dev

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

run() {
  local name="$1"; shift
  echo -e "$YELLOW🔍 $name$NC"
  echo "  $ $*"
  if "$@"; then
    echo -e "  $GREEN✅ $name passed$NC"
  else
    echo -e "  $RED❌ $name failed$NC"
    return 1
  fi
}

echo "🧪 SecurePass 核心验证（无头）"
run "核心单元测试" cargo test --bin securepass_cli --no-default-features

# 前端构建（存在依赖时）
if [ -d frontend/node_modules ]; then
  run "前端构建" bash -c "cd frontend && npm run build"
else
  echo -e "${YELLOW}⏭  跳过前端构建（frontend/node_modules 不存在，请先 cd frontend && npm install）${NC}"
fi

echo -e "$GREEN🎉 完成$NC"
echo "下一步：在带 GUI 依赖的环境运行 'cargo tauri dev' 验证桌面应用；或推送触发 .github/workflows/ci.yml 的三平台构建。"
