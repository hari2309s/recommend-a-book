#!/usr/bin/env bash

# keep-warm.sh - Script to ping the API regularly to prevent cold starts
#
# This script calls the /api/prewarm endpoint to warm up all services
# and prevent cold starts on serverless platforms.
#
# Usage:
#   ./keep-warm.sh [API_URL]
#
# Environment variables:
#   API_URL: The base URL of the API (default: https://recommend-a-book-api.onrender.com)
#   PING_INTERVAL: Time between pings in seconds (default: 600 - 10 minutes)
#   MAX_RETRIES: Maximum number of retry attempts per ping (default: 3)
#   RETRY_DELAY: Delay between retries in seconds (default: 5)
#   LOG_FILE: Path to the log file (default: ./keep-warm.log)
#   VERBOSE: Set to any value to enable verbose logging (default: not set)

# Default configuration
API_URL=${1:-${API_URL:-"https://recommend-a-book-api.onrender.com"}}
PING_INTERVAL=${PING_INTERVAL:-600}  # 10 minutes
MAX_RETRIES=${MAX_RETRIES:-3}
RETRY_DELAY=${RETRY_DELAY:-5}
LOG_FILE=${LOG_FILE:-"./keep-warm.log"}
PREWARM_ENDPOINT="${API_URL}/api/prewarm"
HEALTH_ENDPOINT="${API_URL}/api/health"

# Create log directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Log function
log() {
  local timestamp
  timestamp=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
  echo "[$timestamp] $1" | tee -a "$LOG_FILE"
}

# Ping function with retries
ping_api() {
  local endpoint=$1
  local attempt=1
  local success=false

  while [ $attempt -le $MAX_RETRIES ] && [ "$success" = false ]; do
    if [ -n "$VERBOSE" ]; then
      log "Attempt $attempt: Pinging $endpoint"
    fi

    # Send request and capture HTTP status code
    local response
    local status_code
    response=$(curl -s -w "%{http_code}" -o /tmp/keep-warm-response.txt "$endpoint")
    status_code=$response

    if [ "$status_code" = "200" ] || [ "$status_code" = "201" ]; then
      if [ -n "$VERBOSE" ]; then
        local body
        body=$(cat /tmp/keep-warm-response.txt)
        log "Successful ping to $endpoint (HTTP $status_code)"
        log "Response: $body"
      else
        log "Successfully pinged $endpoint"
      fi
      success=true
      return 0
    else
      if [ $attempt -lt $MAX_RETRIES ]; then
        log "Failed to ping $endpoint (HTTP $status_code). Retrying in ${RETRY_DELAY}s..."
        sleep "$RETRY_DELAY"
      else
        local body
        body=$(cat /tmp/keep-warm-response.txt)
        log "ERROR: Failed to ping $endpoint after $MAX_RETRIES attempts (HTTP $status_code)"
        log "Last response: $body"
      fi
      attempt=$((attempt + 1))
    fi
  done
  return 1
}

# Main execution
log "Starting keep-warm script for $API_URL"
log "Configuration: PING_INTERVAL=${PING_INTERVAL}s, MAX_RETRIES=$MAX_RETRIES, RETRY_DELAY=${RETRY_DELAY}s"

# If running in a one-off mode (not as a daemon)
if [ "$PING_INTERVAL" = "0" ]; then
  log "Running in one-off mode"
  if ping_api "$PREWARM_ENDPOINT"; then
    log "API successfully warmed up"
    exit 0
  else
    log "Falling back to health endpoint"
    if ping_api "$HEALTH_ENDPOINT"; then
      log "API successfully warmed up via health endpoint"
      exit 0
    else
      log "Failed to warm up API"
      exit 1
    fi
  fi
fi

# Running in daemon mode
log "Running in daemon mode, pinging every ${PING_INTERVAL}s. Press Ctrl+C to stop."
while true; do
  # Try the prewarm endpoint first
  if ! ping_api "$PREWARM_ENDPOINT"; then
    log "Prewarm failed, falling back to health endpoint"
    # If prewarm fails, try the health endpoint as a fallback
    if ! ping_api "$HEALTH_ENDPOINT"; then
      log "WARNING: Both endpoints failed. API may be down or experiencing issues."
    fi
  fi

  # Sleep until the next interval
  if [ -n "$VERBOSE" ]; then
    log "Sleeping for ${PING_INTERVAL}s until next ping"
  fi
  sleep "$PING_INTERVAL"
done
