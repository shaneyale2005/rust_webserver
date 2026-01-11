#!/bin/bash

# 压力测试脚本

set -e

echo "================================"
echo "Web 服务器压力测试"
echo "================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 配置
SERVER_URL="http://127.0.0.1:7878"
CONCURRENT_LEVELS=(10 50 100 500 1000)
DURATION=30  # 测试持续时间（秒）

# 检查服务器是否运行
check_server() {
    echo -n "检查服务器是否运行... "
    if curl -s --max-time 2 "$SERVER_URL" > /dev/null 2>&1; then
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

# 检查工具是否安装
check_tool() {
    local tool=$1
    local install_cmd=$2

    if command -v "$tool" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $tool 已安装"
        return 0
    else
        echo -e "${YELLOW}⚠${NC} $tool 未安装"
        echo "  安装命令: $install_cmd"
        return 1
    fi
}

# 使用 curl 进行简单测试
simple_test() {
    echo ""
    echo "=== 简单连接测试 ==="
    echo "发送10个顺序请求..."

    local success=0
    local failed=0
    local total_time=0

    for i in {1..10}; do
        local start=$(date +%s%3N)
        if curl -s --max-time 5 "$SERVER_URL" > /dev/null 2>&1; then
            local end=$(date +%s%3N)
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

# Apache Bench 测试
ab_test() {
    echo ""
    echo "=== Apache Bench 压力测试 ==="

    for concurrent in "${CONCURRENT_LEVELS[@]}"; do
        local requests=$((concurrent * 100))  # 总请求数 = 并发数 * 100

        echo ""
        echo "并发: $concurrent, 总请求: $requests"
        echo "---"

        ab -n "$requests" -c "$concurrent" -q "$SERVER_URL/" 2>&1 | grep -E "(Requests per second|Time per request|Transfer rate|Failed requests)" || true
    done
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

# 并发连接测试
concurrent_curl_test() {
    echo ""
    echo "=== 并发连接测试 (使用 curl) ==="

    local concurrent=100
    echo "同时发起 $concurrent 个并发请求..."

    local success=0
    local failed=0
    local pids=()

    # 启动并发请求
    for i in $(seq 1 $concurrent); do
        curl -s --max-time 10 "$SERVER_URL" > /dev/null 2>&1 &
        pids+=($!)
    done

    # 等待所有请求完成
    for pid in "${pids[@]}"; do
        if wait "$pid" 2>/dev/null; then
            success=$((success + 1))
        else
            failed=$((failed + 1))
        fi
    done

    echo "结果: $success 成功, $failed 失败"
    local success_rate=$((success * 100 / concurrent))
    echo "成功率: ${success_rate}%"
}

# 长时间运行测试
long_running_test() {
    echo ""
    echo "=== 长时间运行测试 ==="
    echo "持续发送请求 60 秒..."

    local start_time=$(date +%s)
    local end_time=$((start_time + 60))
    local count=0
    local errors=0

    while [ $(date +%s) -lt $end_time ]; do
        if curl -s --max-time 2 "$SERVER_URL" > /dev/null 2>&1; then
            count=$((count + 1))
        else
            errors=$((errors + 1))
        fi
        sleep 0.1  # 每100ms一个请求
    done

    echo "总请求数: $count"
    echo "错误数: $errors"
    local rate=$((count / 60))
    echo "平均请求率: ${rate} req/s"
}

# 大文件测试
large_file_test() {
    echo ""
    echo "=== 大文件传输测试 ==="

    # 检查是否有测试文件
    if curl -s --head "$SERVER_URL/test-large.bin" 2>&1 | grep "200 OK" > /dev/null; then
        echo "测试下载大文件 (10次)..."

        local success=0
        local total_size=0

        for i in {1..10}; do
            local output=$(mktemp)
            if curl -s -w "%{size_download}" -o "$output" "$SERVER_URL/test-large.bin" 2>/dev/null); then
                local size=$(stat -f%z "$output" 2>/dev/null || stat -c%s "$output" 2>/dev/null)
                total_size=$((total_size + size))
                success=$((success + 1))
                rm "$output"
            fi
        done

        echo "成功下载: $success/10"
        if [ $success -gt 0 ]; then
            local avg_size=$((total_size / success / 1024))
            echo "平均文件大小: ${avg_size}KB"
        fi
    else
        echo "未找到测试大文件，跳过此测试"
    fi
}

# 内存泄漏检测
memory_leak_test() {
    echo ""
    echo "=== 内存泄漏检测 (简单) ==="
    echo "发送1000个请求，监控服务器进程..."

    # 查找服务器进程
    local pid=$(pgrep -f "target/release/webserver" | head -1)

    if [ -z "$pid" ]; then
        echo "未找到服务器进程，跳过此测试"
        return
    fi

    echo "服务器进程 PID: $pid"

    # 记录初始内存
    local mem_before=$(ps -o rss= -p "$pid" 2>/dev/null || echo "0")
    echo "测试前内存: ${mem_before}KB"

    # 发送大量请求
    for i in $(seq 1 1000); do
        curl -s --max-time 2 "$SERVER_URL" > /dev/null 2>&1 &
        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done
    wait
    echo ""

    # 等待一下让系统稳定
    sleep 2

    # 记录结束内存
    local mem_after=$(ps -o rss= -p "$pid" 2>/dev/null || echo "0")
    echo "测试后内存: ${mem_after}KB"

    local mem_diff=$((mem_after - mem_before))
    echo "内存变化: ${mem_diff}KB"

    if [ "$mem_diff" -gt 10000 ]; then
        echo -e "${YELLOW}⚠ 警告: 内存增长较大，可能存在内存泄漏${NC}"
    else
        echo -e "${GREEN}✓ 内存使用正常${NC}"
    fi
}

# 主函数
main() {
    echo "开始压力测试..."
    echo ""

    # 检查服务器
    check_server

    echo ""
    echo "检查测试工具..."
    local has_ab=$(check_tool "ab" "brew install apache-bench (macOS) 或 apt install apache2-utils (Ubuntu)")
    local has_wrk=$(check_tool "wrk" "brew install wrk (macOS) 或从源码编译")

    echo ""
    echo "================================"
    echo "开始测试..."
    echo "================================"

    # 运行测试
    simple_test
    concurrent_curl_test
    long_running_test
    large_file_test
    memory_leak_test

    if [ $? -eq 0 ]; then
        ab_test
    fi

    if [ $? -eq 0 ]; then
        wrk_test
    fi

    echo ""
    echo "================================"
    echo "压力测试完成！"
    echo "================================"
    echo ""
    echo "建议："
    echo "1. 检查服务器日志中的错误"
    echo "2. 使用 'ps aux | grep webserver' 检查进程状态"
    echo "3. 如果发现性能问题，考虑："
    echo "   - 增加 worker 线程数"
    echo "   - 优化缓存大小"
    echo "   - 使用 perf 或 flamegraph 进行性能分析"
}

# 运行主函数
main "$@"
