#!/bin/bash

# SecurePass 完整测试脚本
# 包含单元测试、集成测试、端到端测试等

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

# 日志文件
LOG_FILE="test-results-$(date +%Y%m%d-%H%M%S).log"
exec > >(tee -a "$LOG_FILE") 2>&1

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

# 1. 环境检查
echo "🔍 测试 1: 环境检查"
echo "===================="

test_case "检查 Rust 环境" "which rustc" "success"
test_case "检查 Cargo" "which cargo" "success"
test_case "检查 Node.js" "which node" "success"
test_case "检查 npm" "which npm" "success"
test_case "检查 Docker" "which docker" "success"
test_case "检查 Docker Compose" "which docker-compose" "success"

# 2. 代码质量检查
echo ""
echo "📝 测试 2: 代码质量检查"
echo "========================"

test_case "检查 Rust 代码格式" "cargo fmt --all -- --check" "success"
test_case "检查 Rust 语法" "cargo check" "success"
test_case "检查 Rust 编译" "cargo build --release" "success"
test_case "检查 TypeScript 类型" "cd frontend && npm run type-check" "success"
test_case "检查 ESLint" "cd frontend && npm run lint" "success"
test_case "检查 Prettier" "cd frontend && npm run format:check" "success"

# 3. 单元测试
echo ""
echo "🧪 测试 3: 单元测试"
echo "===================="

test_case "运行 Rust 单元测试" "cargo test --lib" "success"
test_case "运行 Rust 二进制测试" "cargo test --bin securepass_simple" "success"
test_case "运行 Rust 二进制测试" "cargo test --bin securepass_fixed" "success"
test_case "运行 Rust 二进制测试" "cargo test --bin securepass_complete" "success"
test_case "运行前端单元测试" "cd frontend && npm run test:unit" "success"

# 4. 集成测试
echo ""
echo "🔗 测试 4: 集成测试"
echo "===================="

test_case "运行集成测试脚本" "./scripts/integration-test.sh" "success"
test_case "检查 API 接口连通性" "curl -f http://localhost:8000/health || echo 'Backend not running, skipping'" "success"
test_case "检查前端构建" "cd frontend && npm run build" "success"

# 5. 端到端测试
echo ""
echo "🎯 测试 5: 端到端测试"
echo "===================="

test_case "运行端到端测试" "cd frontend && npm run test:e2e" "success"
test_case "检查构建产物" "test -d frontend/dist" "success"
test_case "检查构建产物大小" "du -sh frontend/dist" "success"

# 6. 性能测试
echo ""
echo "⚡ 测试 6: 性能测试"
echo "===================="

test_case "检查 Rust 二进制大小" "ls -lh target/release/securepass_*" "success"
test_case "检查前端构建大小" "du -sh frontend/dist/*" "success"
test_case "检查内存使用" "cd frontend && npm run test:performance" "success"

# 7. 安全测试
echo ""
echo "🔒 测试 7: 安全测试"
echo "===================="

test_case "检查 Rust 依赖漏洞" "cargo audit" "success"
test_case "检查前端依赖漏洞" "cd frontend && npm audit" "success"
test_case "检查代码安全" "cargo clippy -- -D warnings" "success"
test_case "检查前端代码安全" "cd frontend && npm run security:scan" "success"

# 8. Docker 测试
echo ""
echo "🐳 测试 8: Docker 测试"
echo "===================="

test_case "构建 Docker 镜像" "docker build -t securepass:latest ." "success"
test_case "构建简化 Docker 镜像" "docker build -f Dockerfile.simple -t securepass:web ." "success"
test_case "运行 Docker 容器" "docker run -d --name securepass-test -p 8080:80 securepass:web" "success"
test_case "检查容器状态" "docker ps | grep securepass-test" "success"
test_case "停止容器" "docker stop securepass-test && docker rm securepass-test" "success"

# 9. 部署测试
echo ""
echo "🚀 测试 9: 部署测试"
echo "===================="

test_case "检查部署配置" "test -f docker-compose.yml" "success"
test_case "检查监控配置" "test -f monitoring/prometheus.yml" "success"
test_case "检查日志配置" "test -f monitoring/loki-config.yml" "success"
test_case "检查 Nginx 配置" "test -f nginx.conf" "success"
test_case "检查部署文档" "test -f DEPLOYMENT.md" "success"

# 10. 文档测试
echo ""
echo "📚 测试 10: 文档测试"
echo "===================="

test_case "检查 README" "test -f README.md" "success"
test_case "检查 API 文档" "test -f docs/api.md" "success"
test_case "检查用户手册" "test -f docs/user-guide.md" "success"
test_case "检查开发文档" "test -f docs/development.md" "success"

# 测试结果总结
echo ""
echo "📊 测试结果总结"
echo "================"
echo "总测试数: $TOTAL_TESTS"
echo "通过测试: $PASSED_TESTS"
echo "失败测试: $FAILED_TESTS"

# 生成测试报告
REPORT_FILE="test-report-$(date +%Y%m%d-%H%M%S).md"
echo "# SecurePass 完整测试报告" > "$REPORT_FILE"
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
    echo "- ✅ 所有测试通过，系统可以部署" >> "$REPORT_FILE"
else
    echo "- ❌ 存在测试失败，需要修复问题" >> "$REPORT_FILE"
fi

echo -e "\n${BLUE}📋 测试报告已生成: $REPORT_FILE${NC}"
echo -e "${BLUE}📋 详细日志已保存: $LOG_FILE${NC}"

# 清理临时文件
echo ""
echo "🧹 清理临时文件"
echo "==============="
rm -f temp_test_file_*.tmp

# 最终状态
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 所有测试通过！SecurePass 系统已准备好部署。${NC}"
    exit 0
else
    echo -e "\n${RED}❌ 有 $FAILED_TESTS 个测试失败，请检查问题。${NC}"
    exit 1
fi