#!/usr/bin/env bash
# SessionStart hook: Print current branch context at session start.

set -euo pipefail

BRANCH=$(git branch --show-current 2>/dev/null || echo "detached")
RECENT=$(git log --oneline -3 2>/dev/null || echo "  (no commits)")
UNSTAGED=$(git diff --stat 2>/dev/null | tail -1)
STAGED=$(git diff --cached --stat 2>/dev/null | tail -1)

echo "Branch: $BRANCH"
echo "Recent commits:"
echo "$RECENT"
[ -n "$UNSTAGED" ] && echo "Unstaged: $UNSTAGED"
[ -n "$STAGED" ] && echo "Staged: $STAGED"
