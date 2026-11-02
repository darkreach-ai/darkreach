#!/usr/bin/env bash
# PostToolUse hook (async): Run linters after Write/Edit on source files.
#
# Runs cargo fmt --check for .rs files, tsc --noEmit for .ts/.tsx files.
# Async so it doesn't block the agent.

set -euo pipefail

INPUT=$(cat)

TOOL=$(echo "$INPUT" | jq -r '.tool_name // ""')
if [[ "$TOOL" != "Write" && "$TOOL" != "Edit" ]]; then
    exit 0
fi

FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.filePath // ""')
if [[ -z "$FILE" ]]; then
    exit 0
fi

case "$FILE" in
    *.rs)
        OUTPUT=$(cargo fmt -- --check 2>&1) || {
            echo "cargo fmt check failed for $FILE:"
            echo "$OUTPUT" | head -10
        }
        ;;
    *.ts|*.tsx)
        if [[ "$FILE" == *frontend/* ]]; then
            OUTPUT=$(cd frontend && npx tsc --noEmit 2>&1) || {
                echo "tsc --noEmit failed:"
                echo "$OUTPUT" | head -10
            }
        fi
        ;;
esac
