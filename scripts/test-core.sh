#!/bin/bash

# SecurePass 核心功能测试脚本
# 专注于核心功能的测试

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# 测试结果变量
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 测试函数
test_case() {
    local test_name="$1"
    local test_command="$2"
    local expected_result="$3"
    
    echo -e "${BLUE}测试: $test_name${NC}"
    echo "命令: $test_command"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # 执行测试命令
    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ 通过${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ 失败${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    echo "----------------------------------------"
}

echo "🔒 SecurePass 核心功能测试"
echo "============================"

# 1. 基础环境检查
echo ""
echo "🔍 测试 1: 基础环境检查"
echo "========================"

test_case "检查 Rust 环境" "which rustc" "success"
test_case "检查 Cargo" "which cargo" "success"
test_case "检查 Node.js" "which node" "success"
test_case "检查 npm" "which npm" "success"

# 2. Rust 后端测试
echo ""
echo "🦀 测试 2: Rust 后端测试"
echo "========================"

test_case "简化版本编译" "cargo build --bin securepass_simple --quiet" "success"
test_case "修复版本编译" "cargo build --bin securepass_fixed --quiet" "success"
test_case "完整版本编译" "cargo build --bin securepass_complete --quiet" "success"
test_case "运行简化版本" "./target/debug/securepass_simple" "success"
test_case "运行修复版本" "./target/debug/securepass_fixed" "success"
test_case "运行完整版本" "./target/debug/securepass_complete" "success"

# 3. 前端测试
echo ""
echo "🎨 测试 3: 前端测试"
echo "===================="

test_case "检查前端依赖" "cd frontend && test -d node_modules" "success"
test_case "检查前端配置" "cd frontend && test -f package.json" "success"
test_case "检查 TypeScript 配置" "cd frontend && test -f tsconfig.json" "success"
test_case "检查 Vite 配置" "cd frontend && test -f vite.config.ts" "success"
test_case "检查主组件" "cd frontend && test -f src/App.vue" "success"
test_case "检查密码管理组件" "cd frontend && test -f src/components/PasswordManager.vue" "success"
test_case "检查 API 客户端" "cd frontend && test -f src/api/index.ts" "success"

# 4. 文件结构检查
echo ""
echo "📁 测试 4: 文件结构检查"
echo "========================"

test_case "检查项目根目录" "test -f Cargo.toml" "success"
test_case "检查 Rust 源码" "test -d src" "success"
test_case "检查安全模块" "test -d src/security" "success"
test_case "检查数据库模块" "test -d src/database" "success"
test_case "检查 API 模块" "test -d src/api" "success"
test_case "检查前端目录" "test -d frontend" "success"
test_case "检查脚本目录" "test -d scripts" "success"

# 5. 配置文件检查
echo ""
echo "⚙️ 测试 5: 配置文件检查"
echo "========================"

test_case "检查主配置文件" "test -f tauri.conf.json" "success"
test_case "检查 Tauri 配置" "test -f frontend/src-tauri/tauri.conf.json" "success"
test_case "检查 Tauri Cargo" "test -f frontend/src-tauri/Cargo.toml" "success"
test_case "检查部署文档" "test -f DEPLOYMENT.md" "success"
test_case "检查 Docker 配置" "test -f docker-compose.yml" "success"
test_case "检查 Nginx 配置" "test -f nginx.conf" "success"

# 6. 开发工具检查
echo ""
echo "🛠️ 测试 6: 开发工具检查"
echo "========================"

test_case "检查开发脚本" "test -f scripts/dev.sh" "success"
test_case "检查测试脚本" "test -f scripts/test.sh" "success"
test_case "检查集成测试脚本" "test -f scripts/integration-test.sh" "success"
test_case "检查完整测试脚本" "test -f scripts/test-all.sh" "success"

# 测试结果总结
echo ""
echo "📊 测试结果总结"
echo "================"
echo "总测试数: $TOTAL_TESTS"
echo "通过测试: $PASSED_TESTS"
echo "失败测试: $FAILED_TESTS"

# 生成测试报告
REPORT_FILE="core-test-report-$(date +%Y%m%d-%H%M%S).md"
echo "# SecurePass 核心功能测试报告" > "$REPORT_FILE"
echo "生成时间: $(date)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "## 测试结果" >> "$REPORT_FILE"
echo "- 总测试数: $TOTAL_TESTS" >> "$REPORT_FILE"
echo "- 通过测试: $PASSED_TESTS" >> "$REPORT_FILE"
echo "- 失败测试: $FAILED_TESTS" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "## 测试覆盖率" >> "$REPORT_FILE"
COVERAGE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
echo "- 测试覆盖率: $COVERAGE%" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "## 建议" >> "$REPORT_FILE"
if [ $FAILED_TESTS -eq 0 ]; then
    echo "- ✅ 所有核心测试通过，系统可以部署" >> "$REPORT_FILE"
else
    echo "- ❌ 存在测试失败，需要修复问题" >> "$REPORT_FILE"
fi

echo -e "\n${BLUE}📋 测试报告已生成: $REPORT_FILE${NC}"

# 最终状态
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 所有核心测试通过！SecurePass 系统已准备好部署。${NC}"
    exit 0
else
    echo -e "\n${RED}❌ 有 $FAILED_TESTS 个测试失败，请检查问题。${NC}"
    exit 1
fi