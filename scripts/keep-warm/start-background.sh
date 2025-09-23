#!/usr/bin/env bash

# start-background.sh - Starts the keep-warm script as a background process using nohup
#
# This script runs keep-warm.sh in the background, allowing it to continue running
# after you log out. It stores the PID for later reference and manages log files.
#
# Usage:
#   ./start-background.sh [API_URL] [options]
#
# Options:
#   --status    Check if the background process is running
#   --stop      Stop the running background process
#   --help      Show this help message
#
# Environment variables:
#   All environment variables from keep-warm.sh are supported
#   Plus LOG_DIR to specify a log directory (default: ./logs)

# Default configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
KEEP_WARM_SCRIPT="${SCRIPT_DIR}/keep-warm.sh"
API_URL="${1:-${API_URL:-"https://recommend-a-book-api.onrender.com"}}"
LOG_DIR="${LOG_DIR:-"${SCRIPT_DIR}/logs"}"
PID_FILE="${LOG_DIR}/keep-warm.pid"

# Create log directory if it doesn't exist
mkdir -p "${LOG_DIR}"

# Basic usage function
show_help() {
    echo "Usage: $0 [API_URL] [options]"
    echo ""
    echo "Options:"
    echo "  --status    Check if the background process is running"
    echo "  --stop      Stop the running background process"
    echo "  --help      Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  API_URL     The base URL of your API (default: https://recommend-a-book-api.onrender.com)"
    echo "  PING_INTERVAL Time between pings in seconds (default: 600)"
    echo "  MAX_RETRIES   Maximum number of retry attempts per ping (default: 3)"
    echo "  RETRY_DELAY   Delay between retries in seconds (default: 5)"
    echo "  LOG_DIR       Directory for log files (default: ./logs)"
    echo "  VERBOSE       Set to any value to enable verbose logging"
    echo ""
    echo "Example:"
    echo "  $0                      # Start with default API URL"
    echo "  $0 https://your-api.com # Start with custom API URL"
    echo "  $0 --status             # Check if process is running"
    echo "  $0 --stop               # Stop the process"
}

# Check if the script is running
check_status() {
    if [ -f "${PID_FILE}" ]; then
        PID=$(cat "${PID_FILE}")
        if ps -p "${PID}" > /dev/null; then
            echo "✅ keep-warm script is running with PID ${PID}"
            echo "   Log file: ${LOG_DIR}/keep-warm.log"
            echo "   Process started at: $(ps -p "${PID}" -o lstart=)"
            return 0
        else
            echo "❌ keep-warm script is not running (stale PID file found)"
            echo "   You may want to delete ${PID_FILE} or restart with $0"
            return 1
        fi
    else
        echo "❌ keep-warm script is not running (no PID file)"
        return 1
    fi
}

# Stop the running process
stop_process() {
    if [ -f "${PID_FILE}" ]; then
        PID=$(cat "${PID_FILE}")
        if ps -p "${PID}" > /dev/null; then
            echo "Stopping keep-warm script (PID ${PID})..."
            kill "${PID}"
            sleep 1
            if ps -p "${PID}" > /dev/null; then
                echo "Process still running, sending SIGTERM..."
                kill -TERM "${PID}"
                sleep 2
                if ps -p "${PID}" > /dev/null; then
                    echo "Process still running, sending SIGKILL..."
                    kill -KILL "${PID}"
                fi
            fi
            if ! ps -p "${PID}" > /dev/null; then
                echo "✅ Process stopped successfully"
                rm "${PID_FILE}"
                return 0
            else
                echo "❌ Failed to stop process with PID ${PID}"
                return 1
            fi
        else
            echo "Process with PID ${PID} is not running"
            rm "${PID_FILE}"
            return 0
        fi
    else
        echo "No PID file found, process is not running"
        return 0
    fi
}

# Process command line arguments
if [ "$1" = "--help" ]; then
    show_help
    exit 0
elif [ "$1" = "--status" ]; then
    check_status
    exit $?
elif [ "$1" = "--stop" ]; then
    stop_process
    exit $?
elif [ "$1" = "--restart" ]; then
    stop_process
    # Continue to start a new process
elif [[ "$1" == "http"* ]]; then
    # If the first argument is a URL, use it
    API_URL="$1"
    shift
fi

# Check if the script is already running
if [ -f "${PID_FILE}" ]; then
    PID=$(cat "${PID_FILE}")
    if ps -p "${PID}" > /dev/null; then
        echo "❌ keep-warm script is already running with PID ${PID}"
        echo "   To check status: $0 --status"
        echo "   To stop: $0 --stop"
        echo "   To restart: $0 --restart"
        exit 1
    else
        echo "🔄 Removing stale PID file"
        rm "${PID_FILE}"
    fi
fi

# Check if the keep-warm script exists and is executable
if [ ! -f "${KEEP_WARM_SCRIPT}" ]; then
    echo "❌ Error: keep-warm script not found at ${KEEP_WARM_SCRIPT}"
    exit 1
fi

if [ ! -x "${KEEP_WARM_SCRIPT}" ]; then
    echo "🔄 Making keep-warm script executable"
    chmod +x "${KEEP_WARM_SCRIPT}"
fi

# Start the keep-warm script in the background with nohup
echo "🚀 Starting keep-warm script in the background..."
echo "   API URL: ${API_URL}"
echo "   Log file: ${LOG_DIR}/keep-warm.log"

# Export environment variables
export API_URL
export PING_INTERVAL
export MAX_RETRIES
export RETRY_DELAY
export VERBOSE
export LOG_FILE="${LOG_DIR}/keep-warm.log"

# Start with nohup and save the PID
nohup "${KEEP_WARM_SCRIPT}" > "${LOG_DIR}/keep-warm.log" 2>&1 &
PID=$!
echo ${PID} > "${PID_FILE}"

echo "✅ Process started with PID ${PID}"
echo ""
echo "To check status: $0 --status"
echo "To stop: $0 --stop"
echo "To view logs: tail -f ${LOG_DIR}/keep-warm.log"
