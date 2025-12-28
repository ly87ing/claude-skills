#!/bin/bash
#
# bump-version.sh - Bump version following SemVer
#
# Usage: ./scripts/bump-version.sh <major|minor|patch>
#
# This script increments the version in plugin.json and automatically
# runs sync-version.sh to propagate the change to all related files.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory and plugin root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_JSON="$PLUGIN_DIR/.claude-plugin/plugin.json"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
usage() {
    echo "Usage: $0 <major|minor|patch>"
    echo ""
    echo "Arguments:"
    echo "  major    Increment major version (X.0.0)"
    echo "  minor    Increment minor version (x.Y.0)"
    echo "  patch    Increment patch version (x.y.Z)"
    echo ""
    echo "Example:"
    echo "  $0 patch    # 1.2.3 → 1.2.4"
    echo "  $0 minor    # 1.2.3 → 1.3.0"
    echo "  $0 major    # 1.2.3 → 2.0.0"
    exit 1
}

# Validate argument
if [[ $# -ne 1 ]]; then
    usage
fi

BUMP_TYPE="$1"
if [[ "$BUMP_TYPE" != "major" && "$BUMP_TYPE" != "minor" && "$BUMP_TYPE" != "patch" ]]; then
    log_error "Invalid argument: $BUMP_TYPE"
    usage
fi

# Check plugin.json exists
if [[ ! -f "$PLUGIN_JSON" ]]; then
    log_error "plugin.json not found at: $PLUGIN_JSON"
    exit 1
fi

# Read current version
CURRENT_VERSION=$(grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$PLUGIN_JSON" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')

if [[ -z "$CURRENT_VERSION" ]]; then
    log_error "version field not found in plugin.json"
    exit 1
fi

# Validate current version format
if [[ ! "$CURRENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    log_error "Invalid current version format: $CURRENT_VERSION (expected: MAJOR.MINOR.PATCH)"
    exit 1
fi

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate new version
case "$BUMP_TYPE" in
    major)
        NEW_MAJOR=$((MAJOR + 1))
        NEW_VERSION="${NEW_MAJOR}.0.0"
        ;;
    minor)
        NEW_MINOR=$((MINOR + 1))
        NEW_VERSION="${MAJOR}.${NEW_MINOR}.0"
        ;;
    patch)
        NEW_PATCH=$((PATCH + 1))
        NEW_VERSION="${MAJOR}.${MINOR}.${NEW_PATCH}"
        ;;
esac

echo "=========================================="
echo "  Version Bump"
echo "=========================================="
echo ""
log_info "Bump type: $BUMP_TYPE"
log_info "Current version: $CURRENT_VERSION"
log_info "New version: $NEW_VERSION"
echo ""

# Update plugin.json
log_info "Updating plugin.json..."
sed -i.bak "s/\"version\"[[:space:]]*:[[:space:]]*\"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$PLUGIN_JSON"
rm -f "$PLUGIN_JSON.bak"
log_success "Updated plugin.json"

# Run sync-version.sh
echo ""
log_info "Running sync-version.sh to propagate changes..."
echo ""
"$SCRIPT_DIR/sync-version.sh"

echo ""
echo "=========================================="
echo -e "${GREEN}VERSION BUMP COMPLETE${NC}"
echo "=========================================="
echo ""
echo "  $CURRENT_VERSION → $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  1. Update CHANGELOG.md with new version entry"
echo "  2. Commit changes: git commit -am 'Bump version to $NEW_VERSION'"
echo "  3. Create release: ./scripts/release.sh"
echo "=========================================="
