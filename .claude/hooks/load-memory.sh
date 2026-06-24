#!/bin/bash
# SessionStart hook: inject HANDOFF.md head + INDEX.md head into Claude's context.
# Output is sent as additional context for the session.

set -u

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT" || exit 0

OUTPUT=""

if [ -f "docs/HANDOFF.md" ]; then
    OUTPUT+="# docs/HANDOFF.md (extrait — 80 premières lignes)\n\n"
    OUTPUT+="$(head -n 80 docs/HANDOFF.md)\n\n---\n\n"
fi

if [ -f "docs/INDEX.md" ]; then
    OUTPUT+="# docs/INDEX.md (extrait — 40 premières lignes)\n\n"
    OUTPUT+="$(head -n 40 docs/INDEX.md)\n"
fi

if [ -n "$OUTPUT" ]; then
    jq -nc --arg ctx "$(printf '%b' "$OUTPUT")" \
        '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $ctx}}'
fi

exit 0
