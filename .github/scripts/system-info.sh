#!/bin/bash

# This script was added to help diagnose the issue:
# https://github.com/actions/runner/issues/3724
# It can be safely removed once the issue is resolved.

# Print system information
# Usage: ./system-info.sh "description"

DESCRIPTION=${1:-"System Info"}
BORDER=$(printf '=%.0s' $(seq 1 $((${#DESCRIPTION} + 16))))

echo "=== System Info: $DESCRIPTION ==="
free -h
df -h
ps aux --sort=-%cpu | head -10
echo "$BORDER"
