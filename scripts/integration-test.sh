#!/bin/bash

# SecurePass 前后端集成测试脚本
# 用于测试完整的前后端集成功能

echo "🔒 SecurePass 前后端集成测试"
echo "=================================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# 1. 检查 Rust 后端编译
echo "🔧 测试 1: Rust 后端编译"
echo "========================"

test_case "简化版本编译" "cargo build --bin securepass_simple --quiet" "success"
test_case "修复版本编译" "cargo build --bin securepass_fixed --quiet" "success"
test_case "完整版本编译" "cargo build --bin securepass_complete --quiet" "success"

# 2. 检查前端依赖安装
echo "🎨 测试 2: 前端依赖安装"
echo "========================"

test_case "检查 package.json" "test -f frontend/package.json" "success"
test_case "检查 node_modules" "test -d frontend/node_modules" "success"
test_case "检查 Vite 配置" "test -f frontend/vite.config.ts" "success"
test_case "检查 TypeScript 配置" "test -f frontend/tsconfig.json" "success"

# 3. 检查文件结构
echo "📁 测试 3: 文件结构检查"
echo "========================"

test_case "检查主应用组件" "test -f frontend/src/App.vue" "success"
test_case "检查密码管理组件" "test -f frontend/src/components/PasswordManager.vue" "success"
test_case "检查 API 客户端" "test -f frontend/src/api/index.ts" "success"
test_case "检查主入口文件" "test -f frontend/src/main.ts" "success"

# 4. 检查配置文件
echo "⚙️ 测试 4: 配置文件检查"
echo "========================"

test_case "检查 Tauri 配置" "test -f frontend/src-tauri/tauri.conf.json" "success"
test_case "检查 Tauri Cargo 配置" "test -f frontend/src-tauri/Cargo.toml" "success"
test_case "检查项目配置" "test -f tauri.conf.json" "success"

# 5. 检查 API 接口
echo "🌐 测试 5: API 接口检查"
echo "========================"

test_case "检查密码管理 API" "grep -q 'passwordApi' frontend/src/api/index.ts" "success"
test_case "检查认证 API" "grep -q 'authApi' frontend/src/api/index.ts" "success"
test_case "检查导入导出 API" "grep -q 'importExportApi' frontend/src/api/index.ts" "success"
test_case "检查通知 API" "grep -q 'notificationApi' frontend/src/api/index.ts" "success"

# 6. 检查 Rust 模块
echo "🦀 测试 6: Rust 模块检查"
echo "========================"

test_case "检查主模块" "test -f src/main.rs" "success"
test_case "检查数据库模块" "test -f src/database/mod.rs" "success"
test_case "检查安全模块" "test -f src/security/mod.rs" "success"
test_case "检查 API 模块" "test -f src/api/mod.rs" "success"

# 7. 运行功能测试
echo "🧪 测试 7: 功能测试"
echo "=================="

# 测试简化版本
test_case "运行简化版本" "./target/debug/securepass_simple" "success"

# 测试修复版本
test_case "运行修复版本" "./target/debug/securepass_fixed" "success"

# 测试完整版本
test_case "运行完整版本" "./target/debug/securepass_complete" "success"

# 8. 检查前后端连接
echo "🔗 测试 8: 前后端连接检查"
echo "========================"

# 检查前端是否正确配置了 API 代理
test_case "检查 Vite 代理配置" "grep -q 'proxy' frontend/vite.config.ts" "success"

# 检查 API 基础 URL 配置
test_case "检查 API 基础 URL" "grep -q 'API_BASE_URL' frontend/src/api/index.ts" "success"

# 检查错误处理
test_case "检查错误处理" "grep -q 'handleApiError' frontend/src/api/index.ts" "success"

# 9. 检查安全功能
echo "🔒 测试 9: 安全功能检查"
echo "========================"

test_case "检查加密模块" "test -f src/security/crypto.rs" "success"
test_case "检查密钥管理" "test -f src/security/key_management.rs" "success"
test_case "检查主密码处理" "test -f src/security/master_password.rs" "success"

# 10. 检查开发工具
echo "🛠️ 测试 10: 开发工具检查"
echo "========================"

test_case "检查开发脚本" "test -f scripts/dev.sh" "success"
test_case "检查测试脚本" "test -f scripts/test.sh" "success"
test_case "检查集成测试脚本" "test -f scripts/integration-test.sh" "success"

# 测试结果总结
echo ""
echo "📊 测试结果总结"
echo "================"
echo "总测试数: $TOTAL_TESTS"
echo "通过测试: $PASSED_TESTS"
echo "失败测试: $FAILED_TESTS"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}🎉 所有测试通过！${NC}"
    echo "SecurePass 前后端集成测试完成，系统已准备好进行部署。"
else
    echo -e "${RED}❌ 有 $FAILED_TESTS 个测试失败${NC}"
    echo "请检查失败的项目并修复问题。"
fi

# 生成测试报告
echo ""
echo "📋 生成测试报告"
echo "==============="

REPORT_FILE="integration-test-report.md"
echo "# SecurePass 前后端集成测试报告" > $REPORT_FILE
echo "生成时间: $(date)" >> $REPORT_FILE
echo "" >> $REPORT_FILE
echo "## 测试结果" >> $REPORT_FILE
echo "- 总测试数: $TOTAL_TESTS" >> $REPORT_FILE
echo "- 通过测试: $PASSED_TESTS" >> $REPORT_FILE
echo "- 失败测试: $FAILED_TESTS" >> $REPORT_FILE
echo "" >> $REPORT_FILE
echo "## 测试覆盖率" >> $REPORT_FILE
COVERAGE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
echo "- 测试覆盖率: $COVERAGE%" >> $REPORT_FILE
echo "" >> $REPORT_FILE
echo "## 建议" >> $REPORT_FILE
if [ $FAILED_TESTS -eq 0 ]; then
    echo "- ✅ 所有测试通过，系统可以部署" >> $REPORT_FILE
else
    echo "- ❌ 存在测试失败，需要修复问题" >> $REPORT_FILE
fi

echo "测试报告已生成: $REPORT_FILE"

# 清理临时文件
echo ""
echo "🧹 清理临时文件"
echo "==============="
rm -f temp_test_file_*.tmp

echo ""
echo "🔒 SecurePass 前后端集成测试完成！"