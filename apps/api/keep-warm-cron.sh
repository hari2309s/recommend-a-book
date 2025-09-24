#!/bin/bash
#
# keep-warm-cron.sh
#
# Cron job script to periodically ping the API to keep it warm
# Designed to run on Render.com or any other serverless platform
# to prevent cold starts and timeouts.
#
# Set this script to run every 19 minutes in your hosting platform's
# cron/scheduled task configuration.

# Exit on errors
set -e

# Configuration
API_URL=${API_URL:-"https://recommend-a-book-api.onrender.com"}
ENDPOINTS=("/api/prewarm" "/api/health")
MAX_RETRIES=3
RETRY_DELAY=5
TIMEOUT=30
LOG_FILE="/tmp/keep-warm-cron.log"

# Ensure log file exists
touch $LOG_FILE

# Log function
log() {
  echo "[$(date -u +"%Y-%m-%d %H:%M:%S UTC")] $1" | tee -a $LOG_FILE
}

# Ping function with retries
ping_endpoint() {
  local endpoint=$1
  local full_url="${API_URL}${endpoint}"
  local retries=0

  log "Pinging $full_url..."

  while [[ $retries -lt $MAX_RETRIES ]]; do
    # Use curl with timeout to prevent hanging
    response=$(curl -s -w "%{http_code}" -m $TIMEOUT -o /dev/stderr "$full_url" 2>&1)
    status_code=${response: -3}  # Last 3 characters are the status code
    response_body=${response%???}  # Remove the last 3 characters

    if [[ "$status_code" == "200" ]]; then
      log "✓ Successfully pinged $endpoint (HTTP 200)"
      return 0
    else
      retries=$((retries + 1))
      log "✗ Failed to ping $endpoint (HTTP $status_code). Retry $retries/$MAX_RETRIES in ${RETRY_DELAY}s..."
      sleep $RETRY_DELAY
    fi
  done

  log "ERROR: Failed to ping $endpoint after $MAX_RETRIES attempts. Last status: HTTP $status_code"
  log "Response body: $response_body"
  return 1
}

# Main function
main() {
  log "=== Starting keep-warm cron job for $API_URL ==="

  # Get runtime information
  log "Running on $(hostname) with $(nproc) CPU(s), $(free -h | grep Mem | awk '{print $2}') memory"
  log "Environment: RENDER_SERVICE_ID=${RENDER_SERVICE_ID:-'not on Render'}"

  # Try all endpoints until one succeeds
  for endpoint in "${ENDPOINTS[@]}"; do
    if ping_endpoint "$endpoint"; then
      log "Successfully kept API warm using $endpoint"
      log "=== Keep-warm cron job completed successfully ==="
      exit 0
    fi
  done

  # If we got here, all endpoints failed
  log "WARNING: All endpoints failed. API may be down or experiencing issues."
  log "=== Keep-warm cron job completed with warnings ==="

  # Exit with status code 0 to avoid Render marking the cron job as failed
  # This allows the cron job to continue running on schedule
  exit 0
}

# Run the script
main
