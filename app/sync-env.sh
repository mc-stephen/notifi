#!/usr/bin/env bash
# sync-env.sh — Populate or update .env files for all frontend projects.
# Shared variables are defined here as the single source of truth.
# Run from the app/ directory: ./sync-env.sh

set -euo pipefail

# ── Shared variables (source of truth) ─────────────────────────────────────
# Format: NAME=VALUE
SHARED_VARS=(
  "API_URL=http://localhost:8080"
  "DOCS_URL=http://localhost:8000"
  "STATUS_URL=http://localhost:3001"
  "WEBSITE_URL=http://localhost:3001"
  "GITHUB_URL=https://github.com/notifi"
  "USER_DASHBOARD_URL:https://localhost:3000"
  "ADMIN_DASHBOARD_URL:https://localhost:4321"
)

# ── Helpers ─────────────────────────────────────────────────────────────────

# Detect framework from package.json, return "astro" or "nextjs" or "unknown"
detect_framework() {
  local pkg="$1/package.json"
  if [ ! -f "$pkg" ]; then
    echo "unknown"
    return
  fi
  if grep -q '"astro"' "$pkg" 2>/dev/null; then
    echo "astro"
  elif grep -q '"next"' "$pkg" 2>/dev/null; then
    echo "nextjs"
  else
    echo "unknown"
  fi
}

# Get the prefix for a framework
get_prefix() {
  case "$1" in
    astro)  echo "PUBLIC_" ;;
    nextjs) echo "NEXT_PUBLIC_" ;;
    *)      echo "" ;;
  esac
}

# Get the env filename for a framework
get_env_file() {
  case "$1" in
    astro)  echo ".env" ;;
    nextjs) echo ".env.local" ;;
    *)      echo "" ;;
  esac
}

# Update or append a key=value pair in a file.
# If the key exists (as KEY=), update the line. Otherwise, append.
upsert_env() {
  local file="$1"
  local key="$2"
  local value="$3"

  if grep -q "^${key}=" "$file" 2>/dev/null; then
    # Update existing line
    local tmp
    tmp=$(mktemp)
    sed "s|^${key}=.*|${key}=${value}|" "$file" > "$tmp"
    mv "$tmp" "$file"
  else
    # Append
    echo "${key}=${value}" >> "$file"
  fi
}

# ── Main ───────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CHANGED=0

for project_dir in "$SCRIPT_DIR"/*/; do
  [ -d "$project_dir" ] || continue
  project=$(basename "$project_dir")

  # Skip non-frontend projects (no package.json)
  [ -f "$project_dir/package.json" ] || continue

  framework=$(detect_framework "$project_dir")
  prefix=$(get_prefix "$framework")
  env_file_name=$(get_env_file "$framework")

  if [ -z "$prefix" ]; then
    echo "  skip  $project (not Astro or Next.js)"
    continue
  fi

  env_file="$project_dir$env_file_name"

  # Create the file if it doesn't exist
  if [ ! -f "$env_file" ]; then
    echo "# Auto-managed by sync-env.sh — edit shared vars there instead" > "$env_file"
    echo "  create $project/$env_file_name"
  fi

  # Upsert each shared variable
  for var in "${SHARED_VARS[@]}"; do
    var_name="${var%%=*}"
    var_value="${var#*=}"
    key="${prefix}${var_name}"
    upsert_env "$env_file" "$key" "$var_value"
  done

  echo "  update $project/$env_file_name"
  CHANGED=1
done

if [ "$CHANGED" -eq 0 ]; then
  echo "No frontend projects found."
fi
