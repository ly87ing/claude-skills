#!/bin/bash
#
# update-all.sh - One-click update: bump version, sync, and optionally release
#
# Usage: 
#   ./scripts/update-all.sh <major|minor|patch>           # Bump + sync
#   ./scripts/update-all.sh <major|minor|patch> --release # Bump + sync + release
#   ./scripts/update-all.sh sync                          # Sync only (no bump)
#   ./scripts/update-all.sh validate                      # Validate only
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Script paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd)"

# Helper functions
log_header() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

log_step() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

log_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Show usage
usage() {
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  major              Bump major version (X.0.0) and sync"
    echo "  minor              Bump minor version (x.Y.0) and sync"
    echo "  patch              Bump patch version (x.y.Z) and sync"
    echo "  sync               Sync version to all files (no bump)"
    echo "  validate           Validate version consistency"
    echo ""
    echo "Options:"
    echo "  --release          Also create and push Git tag"
    echo "  --dry-run          Show what would be done without making changes"
    echo ""
    echo "Examples:"
    echo "  $0 patch                    # Bump patch, sync all files"
    echo "  $0 minor --release          # Bump minor, sync, create tag"
    echo "  $0 sync                     # Just sync (after manual edit)"
    echo "  $0 validate                 # Check version consistency"
    exit 1
}

# Parse arguments
if [[ $# -lt 1 ]]; then
    usage
fi

COMMAND="$1"
shift

DO_RELEASE=false
DRY_RUN=false

for arg in "$@"; do
    case $arg in
        --release)
            DO_RELEASE=true
            ;;
        --dry-run)
            DRY_RUN=true
            ;;
        *)
            echo "Unknown option: $arg"
            usage
            ;;
    esac
done

# Main execution
log_header "Java Perf - Update All"

case "$COMMAND" in
    major|minor|patch)
        # Step 1: Bump version
        log_step "Step 1: Bumping $COMMAND version..."
        if [[ "$DRY_RUN" == true ]]; then
            echo "[DRY-RUN] Would run: bump-version.sh $COMMAND"
        else
            "$SCRIPT_DIR/bump-version.sh" "$COMMAND"
        fi
        log_success "Version bumped"

        # Step 2: Validate
        log_step "Step 2: Validating version consistency..."
        if "$REPO_ROOT/scripts/validate-versions.sh" java-perf > /dev/null 2>&1; then
            log_success "All versions consistent"
        else
            log_error "Version mismatch detected!"
            "$REPO_ROOT/scripts/validate-versions.sh" java-perf
            exit 1
        fi

        # Step 3: Release (optional)
        if [[ "$DO_RELEASE" == true ]]; then
            log_step "Step 3: Creating release..."
            if [[ "$DRY_RUN" == true ]]; then
                "$SCRIPT_DIR/release.sh" --dry-run
            else
                "$SCRIPT_DIR/release.sh"
            fi
        fi
        ;;

    sync)
        log_step "Syncing version to all files..."
        if [[ "$DRY_RUN" == true ]]; then
            "$SCRIPT_DIR/sync-version.sh" --dry-run
        else
            "$SCRIPT_DIR/sync-version.sh"
        fi
        ;;

    validate)
        log_step "Validating version consistency..."
        "$REPO_ROOT/scripts/validate-versions.sh" java-perf
        ;;

    *)
        echo "Unknown command: $COMMAND"
        usage
        ;;
esac

log_header "Done!"
