#!/usr/bin/env bash
# PreToolUse hook: Block git push directly to master/main.
#
# Reads the tool use JSON from stdin and checks if it's a Bash command
# that pushes to master or main. Outputs a JSON deny decision if so.

set -euo pipefail

INPUT=$(cat)

TOOL=$(echo "$INPUT" | jq -r '.tool_name // ""')
if [[ "$TOOL" != "Bash" ]]; then
    echo '{"decision": "allow"}'
    exit 0
fi

COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // ""')

# Check for push to master/main (but not push to other branches)
if echo "$COMMAND" | grep -qE 'git\s+push\s+.*\b(master|main)\b'; then
    echo '{"decision": "deny", "reason": "Direct push to master/main is not allowed. Create a PR instead."}'
    exit 0
fi

echo '{"decision": "allow"}'
