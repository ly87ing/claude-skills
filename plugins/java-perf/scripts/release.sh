#!/bin/bash
#
# release.sh - Create and push Git tag for release
#
# Usage: ./scripts/release.sh [--dry-run]
#
# This script creates a Git tag in format <plugin-name>-v<version>
# and pushes it to the remote repository.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory and paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd)"
PLUGIN_JSON="$PLUGIN_DIR/.claude-plugin/plugin.json"

# Parse arguments
DRY_RUN=false
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
    esac
done

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

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check plugin.json exists
if [[ ! -f "$PLUGIN_JSON" ]]; then
    log_error "plugin.json not found at: $PLUGIN_JSON"
    exit 1
fi

# Read plugin name and version
PLUGIN_NAME=$(grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' "$PLUGIN_JSON" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')
VERSION=$(grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$PLUGIN_JSON" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')

if [[ -z "$PLUGIN_NAME" || -z "$VERSION" ]]; then
    log_error "Could not read name or version from plugin.json"
    exit 1
fi

TAG_NAME="${PLUGIN_NAME}-v${VERSION}"

echo "=========================================="
echo "  Release Script"
echo "=========================================="
echo ""

if [[ "$DRY_RUN" == true ]]; then
    log_info "Running in DRY-RUN mode"
    echo ""
fi

log_info "Plugin: $PLUGIN_NAME"
log_info "Version: $VERSION"
log_info "Tag: $TAG_NAME"
echo ""

# Step 1: Validate version consistency
log_info "Validating version consistency..."
if ! "$REPO_ROOT/scripts/validate-versions.sh" "$PLUGIN_NAME" > /dev/null 2>&1; then
    log_error "Version validation failed. Run validate-versions.sh for details."
    log_error "Please run sync-version.sh first to ensure all versions are consistent."
    exit 1
fi
log_success "Version consistency validated"

# Step 2: Check working directory is clean
log_info "Checking working directory..."
if [[ -n $(git -C "$REPO_ROOT" status --porcelain) ]]; then
    log_warning "Working directory has uncommitted changes"
    if [[ "$DRY_RUN" == false ]]; then
        echo ""
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Aborted by user"
            exit 0
        fi
    fi
else
    log_success "Working directory is clean"
fi

# Step 3: Check if tag already exists
log_info "Checking if tag exists..."
if git -C "$REPO_ROOT" tag -l "$TAG_NAME" | grep -q "$TAG_NAME"; then
    log_error "Tag $TAG_NAME already exists"
    log_info "To delete and recreate: git tag -d $TAG_NAME && git push origin :refs/tags/$TAG_NAME"
    exit 1
fi
log_success "Tag $TAG_NAME does not exist"

# Step 4: Check CHANGELOG has entry
log_info "Checking CHANGELOG..."
CHANGELOG="$PLUGIN_DIR/CHANGELOG.md"
if [[ -f "$CHANGELOG" ]]; then
    if grep -q "\[$VERSION\]" "$CHANGELOG"; then
        log_success "CHANGELOG.md has entry for $VERSION"
    else
        log_warning "CHANGELOG.md missing entry for $VERSION"
        if [[ "$DRY_RUN" == false ]]; then
            echo ""
            read -p "Continue without CHANGELOG entry? (y/N) " -n 1 -r
            echo ""
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                log_info "Aborted by user"
                exit 0
            fi
        fi
    fi
else
    log_warning "CHANGELOG.md not found"
fi

echo ""

# Step 5: Create and push tag
if [[ "$DRY_RUN" == true ]]; then
    log_info "[DRY-RUN] Would create tag: $TAG_NAME"
    log_info "[DRY-RUN] Would push tag to origin"
else
    log_info "Creating tag $TAG_NAME..."
    git -C "$REPO_ROOT" tag -a "$TAG_NAME" -m "Release $PLUGIN_NAME v$VERSION"
    log_success "Created tag $TAG_NAME"

    log_info "Pushing tag to origin..."
    git -C "$REPO_ROOT" push origin "$TAG_NAME"
    log_success "Pushed tag $TAG_NAME"
fi

echo ""
echo "=========================================="
if [[ "$DRY_RUN" == true ]]; then
    echo -e "${BLUE}DRY-RUN COMPLETE${NC}"
else
    echo -e "${GREEN}RELEASE COMPLETE${NC}"
fi
echo "=========================================="
echo ""
echo "  Tag: $TAG_NAME"
echo ""
if [[ "$DRY_RUN" == false ]]; then
    echo "View release:"
    echo "  https://github.com/ly87ing/dev-skills/releases/tag/$TAG_NAME"
fi
echo "=========================================="
