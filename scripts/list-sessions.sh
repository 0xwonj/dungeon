#!/usr/bin/env bash
# List all Dungeon save sessions and logs
#
# Automatically reads SAVE_DATA_DIR from .env file if present

set -e

# Load .env file if present to get custom paths
if [ -f .env ]; then
    export $(grep -v '^#' .env | grep -v '^$' | xargs)
fi

# Determine save directory - check .env first, then system defaults
if [ -n "$SAVE_DATA_DIR" ]; then
    # Use SAVE_DATA_DIR from .env
    SAVE_DIR="$SAVE_DATA_DIR"
else
    # Use system default
    if [[ "$OSTYPE" == "darwin"* ]]; then
        SAVE_DIR="$HOME/Library/Application Support/dungeon"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        SAVE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/dungeon"
    else
        SAVE_DIR="$HOME/.local/share/dungeon"
    fi
fi

# Determine log directory (always cache)
if [[ "$OSTYPE" == "darwin"* ]]; then
    LOG_DIR="$HOME/Library/Caches/dungeon/logs"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/dungeon/logs"
else
    LOG_DIR="/tmp/dungeon/logs"
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Dungeon Sessions"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# List save sessions
if [ -d "$SAVE_DIR" ] && [ "$(ls -A "$SAVE_DIR" 2>/dev/null)" ]; then
    echo "📁 Save Data: $SAVE_DIR"
    echo ""
    for session in "$SAVE_DIR"/*; do
        if [ -d "$session" ]; then
            session_id=$(basename "$session")
            echo "  Session: $session_id"

            # Count files in each subdirectory
            for subdir in actions checkpoints events proofs states; do
                if [ -d "$session/$subdir" ]; then
                    count=$(find "$session/$subdir" -type f 2>/dev/null | wc -l | tr -d ' ')
                    if [ "$count" -gt 0 ]; then
                        echo "    - $subdir: $count files"
                    fi
                fi
            done
            echo ""
        fi
    done
else
    echo "📁 No save data found in: $SAVE_DIR"
    echo ""
fi

# List log sessions
if [ -d "$LOG_DIR" ] && [ "$(ls -A "$LOG_DIR" 2>/dev/null)" ]; then
    echo "📝 Logs: $LOG_DIR"
    echo ""
    for session in "$LOG_DIR"/*; do
        if [ -d "$session" ]; then
            session_id=$(basename "$session")
            log_file="$session/client.log"
            if [ -f "$log_file" ]; then
                size=$(du -h "$log_file" | cut -f1)
                lines=$(wc -l < "$log_file" | tr -d ' ')
                echo "  Session: $session_id"
                echo "    - client.log: $size ($lines lines)"
                echo ""
            fi
        fi
    done
else
    echo "📝 No logs found in: $LOG_DIR"
    echo ""
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Commands:"
echo "  Monitor logs:     ./scripts/tail-logs.sh [session_id]"
echo "  Clean old logs:   rm -rf \"$LOG_DIR\"/*"
echo "  Clean save data:  rm -rf \"$SAVE_DIR\"/*"
echo ""
echo "Paths (from .env or system defaults):"
echo "  Save data: $SAVE_DIR"
echo "  Logs:      $LOG_DIR"
