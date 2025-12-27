#!/bin/bash
# sync-version.sh - Sync version from VERSION file to all config files
# Usage: ./scripts/sync-version.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$ROOT_DIR/VERSION"

if [ ! -f "$VERSION_FILE" ]; then
    echo "Error: VERSION file not found at $VERSION_FILE"
    exit 1
fi

VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
echo "Syncing version: $VERSION"

# Update Cargo.toml
CARGO_TOML="$ROOT_DIR/plugins/java-perf/rust/Cargo.toml"
if [ -f "$CARGO_TOML" ]; then
    sed -i '' "s/^version = .*/version = \"$VERSION\"/" "$CARGO_TOML"
    echo "Updated: $CARGO_TOML"
fi

# Update plugin.json
PLUGIN_JSON="$ROOT_DIR/plugins/java-perf/.claude-plugin/plugin.json"
if [ -f "$PLUGIN_JSON" ]; then
    # Use jq if available, otherwise use sed
    if command -v jq &> /dev/null; then
        jq ".version = \"$VERSION\"" "$PLUGIN_JSON" > "${PLUGIN_JSON}.tmp" && mv "${PLUGIN_JSON}.tmp" "$PLUGIN_JSON"
    else
        sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$PLUGIN_JSON"
    fi
    echo "Updated: $PLUGIN_JSON"
fi

# Update marketplace.json
MARKETPLACE_JSON="$ROOT_DIR/.claude-plugin/marketplace.json"
if [ -f "$MARKETPLACE_JSON" ]; then
    if command -v jq &> /dev/null; then
        jq ".plugins[0].version = \"$VERSION\"" "$MARKETPLACE_JSON" > "${MARKETPLACE_JSON}.tmp" && mv "${MARKETPLACE_JSON}.tmp" "$MARKETPLACE_JSON"
    else
        sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$MARKETPLACE_JSON"
    fi
    echo "Updated: $MARKETPLACE_JSON"
fi

echo "Version sync complete: $VERSION"
