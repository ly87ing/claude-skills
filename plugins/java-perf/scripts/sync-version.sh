#!/bin/bash
#
# sync-version.sh - Sync version from plugin.json to all related files
#
# Usage: ./scripts/sync-version.sh [--dry-run]
#
# This script reads the version from plugin.json (single source of truth)
# and updates all related files to maintain version consistency.
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
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd)"

# Files to update
PLUGIN_JSON="$PLUGIN_DIR/.claude-plugin/plugin.json"
CARGO_TOML="$PLUGIN_DIR/rust/Cargo.toml"
README_MD="$PLUGIN_DIR/README.md"
CHANGELOG_MD="$PLUGIN_DIR/CHANGELOG.md"
MARKETPLACE_JSON="$REPO_ROOT/.claude-plugin/marketplace.json"
ROOT_README="$REPO_ROOT/README.md"

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

# Track updated files and warnings
UPDATED_FILES=()
WARNINGS=()

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    WARNINGS+=("$1")
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Validate SemVer format
validate_semver() {
    local version="$1"
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        return 1
    fi
    return 0
}

# Read version from plugin.json
read_plugin_version() {
    if [[ ! -f "$PLUGIN_JSON" ]]; then
        log_error "plugin.json not found at: $PLUGIN_JSON"
        exit 1
    fi

    # Extract version using grep and sed (no jq dependency)
    local version
    version=$(grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$PLUGIN_JSON" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')

    if [[ -z "$version" ]]; then
        log_error "version field not found in plugin.json"
        exit 1
    fi

    if ! validate_semver "$version"; then
        log_error "Invalid version format: $version (expected: MAJOR.MINOR.PATCH)"
        exit 1
    fi

    echo "$version"
}

# Update Cargo.toml version
update_cargo_toml() {
    local version="$1"
    
    if [[ ! -f "$CARGO_TOML" ]]; then
        log_warning "Cargo.toml not found at: $CARGO_TOML"
        return
    fi

    local current_version
    current_version=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/version = "\(.*\)"/\1/')

    if [[ "$current_version" == "$version" ]]; then
        log_info "Cargo.toml already at version $version"
        return
    fi

    if [[ "$DRY_RUN" == true ]]; then
        log_info "[DRY-RUN] Would update Cargo.toml: $current_version → $version"
    else
        sed -i.bak "s/^version = \".*\"/version = \"$version\"/" "$CARGO_TOML"
        rm -f "$CARGO_TOML.bak"
        log_success "Updated Cargo.toml: $current_version → $version"
        UPDATED_FILES+=("rust/Cargo.toml")
    fi
}

# Update README.md title and badge
update_readme() {
    local version="$1"
    
    if [[ ! -f "$README_MD" ]]; then
        log_warning "README.md not found at: $README_MD"
        return
    fi

    local updated=false

    if [[ "$DRY_RUN" == true ]]; then
        # Check if updates would be made
        if grep -q "# Java Perf v[0-9]" "$README_MD"; then
            log_info "[DRY-RUN] Would update README.md title to v$version"
        fi
        if grep -q "Version-[0-9]" "$README_MD"; then
            log_info "[DRY-RUN] Would update README.md badge to $version"
        fi
    else
        # Update title (# Java Perf vX.Y.Z)
        if grep -q "# Java Perf v[0-9]" "$README_MD"; then
            sed -i.bak "s/# Java Perf v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/# Java Perf v$version/" "$README_MD"
            updated=true
        fi

        # Update badge (Version-X.Y.Z-blue)
        if grep -q "Version-[0-9]" "$README_MD"; then
            sed -i.bak "s/Version-[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/Version-$version/" "$README_MD"
            updated=true
        fi

        rm -f "$README_MD.bak"

        if [[ "$updated" == true ]]; then
            log_success "Updated README.md to version $version"
            UPDATED_FILES+=("README.md")
        else
            log_info "README.md already at version $version"
        fi
    fi
}

# Update marketplace.json plugin entry
update_marketplace_json() {
    local version="$1"
    local plugin_name="java-perf"
    
    if [[ ! -f "$MARKETPLACE_JSON" ]]; then
        log_warning "marketplace.json not found at: $MARKETPLACE_JSON"
        return
    fi

    # Check current version in marketplace.json for this plugin
    local current_version
    current_version=$(grep -A5 "\"name\": \"$plugin_name\"" "$MARKETPLACE_JSON" | grep '"version"' | head -1 | sed 's/.*"\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)".*/\1/')

    if [[ "$current_version" == "$version" ]]; then
        log_info "marketplace.json already at version $version"
        return
    fi

    if [[ "$DRY_RUN" == true ]]; then
        log_info "[DRY-RUN] Would update marketplace.json: $current_version → $version"
    else
        # Use sed to update the version - compatible with both macOS and Linux
        # Find the line with the old version and replace it
        sed -i.bak "s/\"version\": \"$current_version\"/\"version\": \"$version\"/" "$MARKETPLACE_JSON"
        rm -f "$MARKETPLACE_JSON.bak"
        log_success "Updated marketplace.json: $current_version → $version"
        UPDATED_FILES+=("../../.claude-plugin/marketplace.json")
    fi
}

# Update root README.md plugin table
update_root_readme() {
    local version="$1"
    
    if [[ ! -f "$ROOT_README" ]]; then
        log_warning "Root README.md not found at: $ROOT_README"
        return
    fi

    # Check if java-perf row exists and get current version
    local current_version
    current_version=$(grep -o "java-perf.*|[[:space:]]*[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*" "$ROOT_README" | grep -o "[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$")

    if [[ "$current_version" == "$version" ]]; then
        log_info "Root README.md already at version $version"
        return
    fi

    if [[ "$DRY_RUN" == true ]]; then
        log_info "[DRY-RUN] Would update root README.md table: $current_version → $version"
    else
        # Update the version in the java-perf table row
        sed -i.bak "s/\(java-perf.*|\)[[:space:]]*[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/\1 $version/" "$ROOT_README"
        rm -f "$ROOT_README.bak"
        log_success "Updated root README.md table: $current_version → $version"
        UPDATED_FILES+=("../../README.md")
    fi
}

# Validate CHANGELOG.md contains current version entry
validate_changelog() {
    local version="$1"
    
    if [[ ! -f "$CHANGELOG_MD" ]]; then
        log_warning "CHANGELOG.md not found at: $CHANGELOG_MD"
        return
    fi

    if grep -q "\[$version\]" "$CHANGELOG_MD"; then
        log_success "CHANGELOG.md contains entry for version $version"
    else
        log_warning "CHANGELOG.md does not contain entry for version $version"
    fi
}

# Print summary
print_summary() {
    echo ""
    echo "=========================================="
    
    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${BLUE}DRY-RUN SUMMARY${NC}"
    else
        echo -e "${GREEN}SYNC SUMMARY${NC}"
    fi
    
    echo "=========================================="
    echo "Version: $VERSION"
    echo ""

    if [[ ${#UPDATED_FILES[@]} -gt 0 ]]; then
        echo "Updated files:"
        for file in "${UPDATED_FILES[@]}"; do
            echo "  ✓ $file"
        done
    else
        if [[ "$DRY_RUN" == true ]]; then
            echo "No files would be updated."
        else
            echo "No files needed updating."
        fi
    fi

    if [[ ${#WARNINGS[@]} -gt 0 ]]; then
        echo ""
        echo "Warnings:"
        for warning in "${WARNINGS[@]}"; do
            echo "  ⚠ $warning"
        done
    fi

    echo "=========================================="
}

# Main execution
main() {
    echo "=========================================="
    echo "  Version Sync Script"
    echo "=========================================="
    echo ""

    if [[ "$DRY_RUN" == true ]]; then
        log_info "Running in DRY-RUN mode (no files will be modified)"
        echo ""
    fi

    # Read version from plugin.json
    VERSION=$(read_plugin_version)
    log_info "Source version (plugin.json): $VERSION"
    echo ""

    # Update all target files
    update_cargo_toml "$VERSION"
    update_readme "$VERSION"
    update_marketplace_json "$VERSION"
    update_root_readme "$VERSION"
    
    echo ""
    # Validate CHANGELOG
    validate_changelog "$VERSION"

    # Print summary
    print_summary
}

main "$@"
