#!/usr/bin/env bash
# Monitor Dungeon client logs in real-time
#
# Automatically reads configuration from .env file if present

set -e

# Load .env file if present to get custom paths
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep -v '^$' | xargs)
fi

# Determine log directory based on OS (logs always go to cache)
if [[ "$OSTYPE" == "darwin"* ]]; then
    LOG_DIR="$HOME/Library/Caches/dungeon/logs"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/dungeon/logs"
else
    LOG_DIR="/tmp/dungeon/logs"
fi

# Check if log directory exists
if [ ! -d "$LOG_DIR" ]; then
    echo "❌ Log directory not found: $LOG_DIR"
    echo "   Run the client first to generate logs."
    exit 1
fi

# Check if any log files exist
if ! ls "$LOG_DIR"/*/client.log 1> /dev/null 2>&1; then
    echo "❌ No log files found in: $LOG_DIR"
    echo "   Run the client first to generate logs."
    exit 1
fi

# If specific session provided as argument, tail that session
if [ -n "$1" ]; then
    SESSION_LOG="$LOG_DIR/$1/client.log"
    if [ ! -f "$SESSION_LOG" ]; then
        echo "❌ Session log not found: $SESSION_LOG"
        echo ""
        echo "Available sessions:"
        ls -1t "$LOG_DIR"
        exit 1
    fi
    echo "📝 Monitoring session: $1"
    echo "   Log file: $SESSION_LOG"
    echo ""
    tail -f "$SESSION_LOG"
else
    # Find the most recent log file
    LATEST_LOG=$(ls -t "$LOG_DIR"/*/client.log | head -1)
    SESSION_ID=$(basename "$(dirname "$LATEST_LOG")")

    echo "📝 Monitoring latest session: $SESSION_ID"
    echo "   Log file: $LATEST_LOG"
    echo ""
    echo "Tip: To monitor a specific session, run: $0 <session_id>"
    echo ""
    tail -f "$LATEST_LOG"
fi
