#!/bin/bash

# 压力测试脚本 - HTTP/1.1

set -e

echo "================================"
echo "Web 服务器压力测试 (HTTP/1.1)"
echo "================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# 配置
SERVER_URL="http://127.0.0.1:7878"
CONCURRENT_LEVELS=(10 50 100 500 1000 10000)
DURATION=30

# 检查服务器是否运行
check_server() {
    echo -n "检查服务器是否运行... "
    if curl -s --http1.1 --max-time 2 "$SERVER_URL" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} 服务器正在运行"
        return 0
    else
        echo -e "${RED}✗${NC} 服务器未运行"
        echo ""
        echo "请先启动服务器："
        echo "  cargo run --release"
        echo ""
        exit 1
    fi
}

# 简单测试
simple_test() {
    echo ""
    echo "=== 简单连接测试 (10个请求) ==="

    local success=0
    local failed=0
    local total_time=0

    for i in {1..10}; do
        local start=$(python3 -c "import time; print(int(time.time() * 1000))")
        if curl -s --http1.1 --max-time 5 "$SERVER_URL" > /dev/null 2>&1; then
            local end=$(python3 -c "import time; print(int(time.time() * 1000))")
            local duration=$((end - start))
            total_time=$((total_time + duration))
            success=$((success + 1))
            echo -e "  请求 $i: ${GREEN}成功${NC} (${duration}ms)"
        else
            failed=$((failed + 1))
            echo -e "  请求 $i: ${RED}失败${NC}"
        fi
    done

    echo ""
    echo "结果: $success 成功, $failed 失败"
    if [ $success -gt 0 ]; then
        local avg=$((total_time / success))
        echo "平均响应时间: ${avg}ms"
    fi
}

# wrk 测试
wrk_test() {
    echo ""
    echo "=== wrk 压力测试 ==="

    for concurrent in "${CONCURRENT_LEVELS[@]}"; do
        echo ""
        echo "并发连接: $concurrent, 持续时间: ${DURATION}秒"
        echo "---"

        wrk -t 4 -c "$concurrent" -d "${DURATION}s" --latency "$SERVER_URL/" 2>&1 || true
    done
}

# 主函数
main() {
    echo "开始压力测试..."
    echo ""

    # 检查服务器
    check_server

    echo ""
    echo "================================"
    echo "开始测试..."
    echo "================================"

    # 运行测试
    simple_test
    wrk_test
}

# 运行主函数
main "$@"
