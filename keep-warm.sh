#!/bin/bash
#
# Keep-Warm Script for Recommend-a-Book API
#
# This script periodically pings the API to keep it warm and prevent cold starts
# on serverless platforms like Render.com or Vercel. It works by calling the prewarm
# endpoint at regular intervals.
#
# Usage:
#   ./keep-warm.sh [options]
#
# Options:
#   --url URL               The API URL to ping (default: https://recommend-a-book-api.onrender.com)
#   --interval SECONDS      Ping interval in seconds (default: 600 - 10 minutes)
#   --max-retries NUMBER    Max number of retries on failure (default: 3)
#   --retry-delay SECONDS   Delay between retries in seconds (default: 5)
#   --daemon                Run in background as daemon
#   --log-file FILE         Log file (default: ./keep-warm.log)
#   --help                  Show this help message
#
# Example:
#   ./keep-warm.sh --url https://your-api.example.com --interval 300 --daemon

set -e

# Default configuration
API_URL="https://recommend-a-book-api.onrender.com"
PING_INTERVAL=1140  # 19 minutes
MAX_RETRIES=3
RETRY_DELAY=5
DAEMON_MODE=false
LOG_FILE="./keep-warm.log"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --url)
      API_URL="$2"
      shift 2
      ;;
    --interval)
      PING_INTERVAL="$2"
      shift 2
      ;;
    --max-retries)
      MAX_RETRIES="$2"
      shift 2
      ;;
    --retry-delay)
      RETRY_DELAY="$2"
      shift 2
      ;;
    --daemon)
      DAEMON_MODE=true
      shift
      ;;
    --log-file)
      LOG_FILE="$2"
      shift 2
      ;;
    --help)
      echo "Usage: $0 [options]"
      echo "Options:"
      echo "  --url URL               The API URL to ping (default: https://recommend-a-book-api.onrender.com)"
      echo "  --interval SECONDS      Ping interval in seconds (default: 1140 - 19 minutes)"
      echo "  --max-retries NUMBER    Max number of retries on failure (default: 3)"
      echo "  --retry-delay SECONDS   Delay between retries in seconds (default: 5)"
      echo "  --daemon                Run in background as daemon"
      echo "  --log-file FILE         Log file (default: ./keep-warm.log)"
      echo "  --help                  Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help to see available options"
      exit 1
      ;;
  esac
done

# Function to log messages
log() {
  local timestamp=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
  echo "[$timestamp] $1"
  echo "[$timestamp] $1" >> "$LOG_FILE"
}

# Function to ping API with retries
ping_api() {
  local endpoint=$1
  local full_url="${API_URL}${endpoint}"
  local retries=0

  while [[ $retries -lt $MAX_RETRIES ]]; do
    local response
    local status_code

    # Execute the curl command and capture both response and status code
    response=$(curl -s -w "%{http_code}" -o /dev/stderr "$full_url" 2>&1)
    status_code=${response: -3}  # Extract the last 3 characters (status code)
    response=${response%???}     # Remove the last 3 characters to get the response body

    if [[ "$status_code" == "200" ]]; then
      log "Successfully pinged $full_url"
      return 0
    else
      retries=$((retries + 1))
      log "Failed to ping $full_url (HTTP $status_code). Retrying in ${RETRY_DELAY}s..."
      sleep "$RETRY_DELAY"
    fi
  done

  log "ERROR: Failed to ping $full_url after $MAX_RETRIES attempts (HTTP $status_code)"
  log "Last response: $response"
  return 1
}

# Function to warm up API
warm_up() {
  # Try prewarm endpoint first
  if ping_api "/api/prewarm"; then
    return 0
  fi

  log "Prewarm failed, falling back to health endpoint"

  # Fall back to health endpoint
  if ping_api "/api/health"; then
    return 0
  fi

  log "WARNING: Both endpoints failed. API may be down or experiencing issues."
  return 1
}

# Main function
main() {
  log "Starting keep-warm script for $API_URL"
  log "Configuration: PING_INTERVAL=${PING_INTERVAL}s, MAX_RETRIES=$MAX_RETRIES, RETRY_DELAY=${RETRY_DELAY}s"

  # Setup trap to handle script termination
  trap "log 'Keep-warm script stopped.'; exit 0" SIGINT SIGTERM

  if $DAEMON_MODE; then
    log "Running in daemon mode, pinging every ${PING_INTERVAL}s. Press Ctrl+C to stop."

    while true; do
      warm_up
      sleep "$PING_INTERVAL"
    done
  else
    log "Running in one-shot mode."
    warm_up
  fi
}

# Run the script
main
