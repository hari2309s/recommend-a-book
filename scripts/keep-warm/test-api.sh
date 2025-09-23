#!/usr/bin/env bash

# test-api.sh - Script to test API endpoints and verify they're working
#
# This script tests the prewarm and health endpoints of the API to ensure
# they're responding correctly, with detailed output about the response.
#
# Usage:
#   ./test-api.sh [API_URL]
#
# Environment variables:
#   API_URL: The base URL of the API (default: https://recommend-a-book-api.onrender.com)

# Default configuration
API_URL=${1:-${API_URL:-"https://recommend-a-book-api.onrender.com"}}
PREWARM_ENDPOINT="${API_URL}/api/prewarm"
HEALTH_ENDPOINT="${API_URL}/api/health"

# Colors for better output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RESET='\033[0m'
BOLD='\033[1m'

# Print header
echo -e "\n${BOLD}===== API Endpoint Test =====\n${RESET}"
echo -e "${BLUE}Testing API endpoints for:${RESET} ${API_URL}"
echo -e "${BLUE}Time:${RESET} $(date -u)"
echo -e "${BLUE}Test started...${RESET}\n"

# Function to test an endpoint
test_endpoint() {
  local endpoint=$1
  local name=$2
  local temp_file=$(mktemp)

  echo -e "${BOLD}Testing ${name} endpoint:${RESET} ${endpoint}"

  # Make the request with timing information
  echo -e "${BLUE}Sending request...${RESET}"

  # Create separate files for response body and response info
  local response_body_file=$(mktemp)
  local response_info_file=$(mktemp)

  # Use curl to make the request and capture timing information
  curl -s -w "%{http_code},%{time_total},%{time_connect},%{time_starttransfer}" \
    -o "$response_body_file" \
    --connect-timeout 10 \
    "$endpoint" > "$response_info_file"

  # Extract the timing information from the response
  IFS=',' read -r http_code time_total time_connect time_starttransfer < "$response_info_file"

  # Read the response body
  response=$(cat "$response_body_file")

  # Clean up
  rm "$response_body_file" "$response_info_file"

  # Output debug information
  echo -e "${BLUE}HTTP Status:${RESET} $http_code"
  echo -e "${BLUE}Response Time:${RESET} ${time_total}s"

  # Determine if the request was successful
  if [[ "$http_code" == "200" || "$http_code" == "201" ]]; then
    echo -e "${GREEN}✓ Success (HTTP ${http_code})${RESET}"

    # Try to pretty-print JSON if jq is available
    if command -v jq &>/dev/null && [[ "$response" == "{"* ]]; then
      echo -e "${BLUE}Response:${RESET}"
      echo "$response" | jq .
    else
      echo -e "${BLUE}Response:${RESET} ${response}"
    fi

    echo -e "${BLUE}Time Total:${RESET} ${time_total}s"

    # Check if the response time indicates a cold start
    cold_start="no"
    if (( $(echo "$time_starttransfer > 1.0" | bc -l 2>/dev/null) )); then
      cold_start="likely yes (>1s)"
    fi

    echo -e "${BLUE}Cold Start:${RESET} ${cold_start}"
    return 0
  else
    echo -e "${RED}✗ Failed (HTTP ${http_code})${RESET}"
    echo -e "${BLUE}Response:${RESET} ${response}"
    return 1
  fi
}

# Test the prewarm endpoint
echo -e "${YELLOW}${BOLD}STEP 1: Testing prewarm endpoint${RESET}"
if test_endpoint "$PREWARM_ENDPOINT" "prewarm"; then
  prewarm_success=true
else
  prewarm_success=false
  echo -e "${YELLOW}Prewarm endpoint failed. This could indicate an API issue.${RESET}\n"
fi

echo

# Test the health endpoint
echo -e "${YELLOW}${BOLD}STEP 2: Testing health endpoint${RESET}"
if test_endpoint "$HEALTH_ENDPOINT" "health"; then
  health_success=true
else
  health_success=false
  echo -e "${YELLOW}Health endpoint failed. This could indicate an API issue.${RESET}\n"
fi

echo

# Summary
echo -e "${BOLD}===== TEST SUMMARY =====\n${RESET}"
echo -e "API URL: ${API_URL}"

if [[ "$prewarm_success" == "true" ]]; then
  echo -e "Prewarm Endpoint: ${GREEN}✓ Success${RESET}"
else
  echo -e "Prewarm Endpoint: ${RED}✗ Failed${RESET}"
fi

if [[ "$health_success" == "true" ]]; then
  echo -e "Health Endpoint: ${GREEN}✓ Success${RESET}"
else
  echo -e "Health Endpoint: ${RED}✗ Failed${RESET}"
fi

if [[ "$prewarm_success" == "true" || "$health_success" == "true" ]]; then
  echo -e "\n${GREEN}${BOLD}API is responsive!${RESET}"
  echo -e "The keep-warm system should function correctly."
  exit 0
else
  echo -e "\n${RED}${BOLD}API is not responding correctly.${RESET}"
  echo -e "Please check your API deployment status and configuration."
  exit 1
fi
