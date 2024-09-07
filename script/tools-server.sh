#!/bin/bash

DIR="$(cd "$(dirname "$0")" && cd .. && pwd)"

# 定义要启动的程序及其参数
LOGFILE="$DIR/log/server.log"
PROGRAM="tools-server"

# 启动程序的函数
start_program() {
    echo "" >> "$LOGFILE"
    echo "" >> "$LOGFILE"
    echo "" >> "$LOGFILE"
    echo "=== Starting $PROGRAM... $(date)" | tee -a "$LOGFILE"
    cd $DIR/server && cargo run >> "$LOGFILE" 2>&1 &
    program_pid=$!
    echo "Program started with PID $program_pid"
}

# 重启程序的函数
restart_program() {
    echo "=== Restarting $PROGRAM..."
    echo "pkill -P $program_pid"
    pkill -P $program_pid
    sleep 1
    start_program
}

# 捕捉 SIGHUP 信号并重启程序
trap 'restart_program' SIGUSR1

# 启动程序
start_program

# 等待程序结束
echo "Script is running. Waiting for signals..."
while true; do
    sleep 1  # 等待信号的同时减少 CPU 占用
done
