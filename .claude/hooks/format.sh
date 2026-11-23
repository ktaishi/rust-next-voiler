#!/usr/bin/env bash
# PostToolUse hook: Edit/Write されたファイルを自動整形する (失敗しても作業は止めない)
set -u
file=$(jq -r '.tool_input.file_path // empty' 2>/dev/null)
if [ -z "$file" ] || [ ! -f "$file" ]; then
  exit 0
fi
root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
case "$file" in
  "$root"/backend/*.rs)
    rustfmt --edition 2021 "$file" >/dev/null 2>&1 ;;
  "$root"/frontend/*.ts|"$root"/frontend/*.tsx|"$root"/frontend/*.css|"$root"/frontend/*.json|"$root"/frontend/*.mjs)
    if [ -x "$root/frontend/node_modules/.bin/prettier" ]; then
      (cd "$root/frontend" && ./node_modules/.bin/prettier --write "$file" >/dev/null 2>&1)
    fi ;;
  *.tf|*.tfvars)
    if command -v terraform >/dev/null 2>&1; then
      terraform fmt "$file" >/dev/null 2>&1
    fi ;;
esac
exit 0
