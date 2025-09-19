#!/bin/bash

# Debug version to identify where the script hangs
set -euo pipefail

echo "Debug: Script started"

CONFIG_FILE="../config/dev.toml"

echo "Debug: About to parse config"

# Test the config parsing separately
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Config file not found"
    exit 1
fi

echo "Debug: Config file exists"

# Test the grep commands that were causing issues
echo "Debug: Testing grep commands"
DB_HOST=$(grep -A 10 "^\[server\]" "$CONFIG_FILE" | grep "^host" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?.*/\1/' | tr -d ' ')
echo "Debug: DB_HOST extracted: '$DB_HOST'"

DB_PORT=$(grep -A 10 "^\[server\]" "$CONFIG_FILE" | grep "^port" | head -1 | sed 's/.*=\s*\([0-9]*\).*/\1/')
echo "Debug: DB_PORT extracted: '$DB_PORT'"

# Set defaults
if [[ -z "$DB_HOST" || "$DB_HOST" == "0.0.0.0" ]]; then
    DB_HOST="localhost"
    echo "Debug: Using localhost for host"
fi
if [[ -z "$DB_PORT" ]]; then
    DB_PORT="8080"
    echo "Debug: Using default port 8080"
fi

API_ENDPOINT="http://${DB_HOST}:${DB_PORT}"
echo "Debug: API_ENDPOINT set to: $API_ENDPOINT"

echo "Debug: About to test curl"

# Test curl without measure_time function
curl -s -w "%{time_total}" -o /dev/null "$API_ENDPOINT/health" 2>/dev/null || echo "Debug: Curl failed"

echo "Debug: Curl test completed"
echo "Debug: Script completed successfully"
