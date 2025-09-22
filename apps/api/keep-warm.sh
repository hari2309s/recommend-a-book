#!/bin/bash
# keep-warm.sh - Script to prevent cold starts on Render by periodically hitting the API
#
# Usage:
#   ./keep-warm.sh [API_URL]
#   Example: ./keep-warm.sh https://recommend-a-book-rust-api.onrender.com
#
# Make executable: chmod +x keep-warm.sh
#
# This script sends periodic requests to:
# - The /api/prewarm endpoint to initialize services
# - The /api/health endpoint as a fallback

set -e

# Default API URL (override with first parameter)
API_URL=${1:-"https://recommend-a-book-rust-api.onrender.com"}
PREWARM_ENDPOINT="${API_URL}/api/prewarm"
HEALTH_ENDPOINT="${API_URL}/api/health"
INTERVAL_MINUTES=${KEEP_WARM_INTERVAL:-15}  # Minutes between requests, can be set via env var
MAX_RETRIES=3
TIMEOUT_SECONDS=30

echo "🔥 Starting keep-warm script for ${API_URL}"
echo "🕒 Will ping API every ${INTERVAL_MINUTES} minutes"

# Convert minutes to seconds for sleep
INTERVAL_SECONDS=$((INTERVAL_MINUTES * 60))

# Function to make a request with retries
make_request() {
    local endpoint=$1
    local retry_count=0
    local success=false

    echo "$(date '+%Y-%m-%d %H:%M:%S') - Pinging ${endpoint}..."

    while [ $retry_count -lt $MAX_RETRIES ] && [ "$success" = false ]; do
        if curl -s -f -m $TIMEOUT_SECONDS "${endpoint}" > /dev/null 2>&1; then
            echo "✅ Success! API is warm at ${endpoint}"
            success=true
            return 0
        else
            retry_count=$((retry_count + 1))
            if [ $retry_count -lt $MAX_RETRIES ]; then
                echo "⚠️ Request failed. Retrying in 10 seconds... (Attempt ${retry_count}/${MAX_RETRIES})"
                sleep 10
            else
                echo "❌ Failed to connect to ${endpoint} after ${MAX_RETRIES} attempts"
            fi
        fi
    done

    return 1
}

# Main loop
while true; do
    echo "-------------------------------------------"
    echo "🔄 Keep-warm cycle started at $(date '+%Y-%m-%d %H:%M:%S')"

    # Try prewarm endpoint first
    if make_request "${PREWARM_ENDPOINT}"; then
        echo "🚀 Prewarming completed successfully"
    else
        # Fall back to health endpoint
        echo "⚠️ Prewarming failed, falling back to health check..."
        if make_request "${HEALTH_ENDPOINT}"; then
            echo "🩺 Health check passed"
        else
            echo "❌ All endpoints failed - API may be down"
        fi
    fi

    echo "💤 Sleeping for ${INTERVAL_MINUTES} minutes until next ping..."
    echo "-------------------------------------------"
    sleep ${INTERVAL_SECONDS}
done
