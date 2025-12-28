#!/bin/bash
#
# validate-versions.sh - Validate version consistency across all files
#
# Usage: ./scripts/validate-versions.sh [plugin-name]
#
# This script checks that all version references match the plugin.json version.
# Used by CI to catch version mismatches before merge.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory and repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Track errors
ERRORS=()
CHECKS_PASSED=0
CHECKS_FAILED=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((CHECKS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ERRORS+=("$1")
    ((CHECKS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Validate a single plugin
validate_plugin() {
    local plugin_name="$1"
    local plugin_dir="$REPO_ROOT/plugins/$plugin_name"
    local plugin_json="$plugin_dir/.claude-plugin/plugin.json"

    echo ""
    echo "=========================================="
    echo "  Validating: $plugin_name"
    echo "=========================================="
    echo ""

    # Check plugin.json exists
    if [[ ! -f "$plugin_json" ]]; then
        log_error "plugin.json not found: $plugin_json"
        return 1
    fi

    # Read expected version from plugin.json
    local expected_version
    expected_version=$(grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$plugin_json" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')

    if [[ -z "$expected_version" ]]; then
        log_error "version field not found in plugin.json"
        return 1
    fi

    log_info "Expected version (plugin.json): $expected_version"
    echo ""

    # Check Cargo.toml
    local cargo_toml="$plugin_dir/rust/Cargo.toml"
    if [[ -f "$cargo_toml" ]]; then
        local cargo_version
        cargo_version=$(grep -m1 '^version = ' "$cargo_toml" | sed 's/version = "\(.*\)"/\1/')
        
        if [[ "$cargo_version" == "$expected_version" ]]; then
            log_success "Cargo.toml: $cargo_version"
        else
            log_error "Cargo.toml: expected $expected_version, got $cargo_version"
        fi
    else
        log_warning "Cargo.toml not found (skipped)"
    fi

    # Check README.md badge
    local readme_md="$plugin_dir/README.md"
    if [[ -f "$readme_md" ]]; then
        local badge_version
        badge_version=$(grep -o "Version-[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*" "$readme_md" | head -1 | sed 's/Version-//')
        
        if [[ -z "$badge_version" ]]; then
            log_warning "README.md badge version not found (skipped)"
        elif [[ "$badge_version" == "$expected_version" ]]; then
            log_success "README.md badge: $badge_version"
        else
            log_error "README.md badge: expected $expected_version, got $badge_version"
        fi

        # Check README.md title
        local title_version
        title_version=$(grep -o "# Java Perf v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*" "$readme_md" | head -1 | sed 's/# Java Perf v//')
        
        if [[ -z "$title_version" ]]; then
            log_warning "README.md title version not found (skipped)"
        elif [[ "$title_version" == "$expected_version" ]]; then
            log_success "README.md title: $title_version"
        else
            log_error "README.md title: expected $expected_version, got $title_version"
        fi
    else
        log_warning "README.md not found (skipped)"
    fi

    # Check marketplace.json
    local marketplace_json="$REPO_ROOT/.claude-plugin/marketplace.json"
    if [[ -f "$marketplace_json" ]]; then
        local marketplace_version
        marketplace_version=$(grep -A5 "\"name\": \"$plugin_name\"" "$marketplace_json" | grep '"version"' | head -1 | sed 's/.*"\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)".*/\1/')
        
        if [[ -z "$marketplace_version" ]]; then
            log_warning "marketplace.json entry for $plugin_name not found (skipped)"
        elif [[ "$marketplace_version" == "$expected_version" ]]; then
            log_success "marketplace.json: $marketplace_version"
        else
            log_error "marketplace.json: expected $expected_version, got $marketplace_version"
        fi
    else
        log_warning "marketplace.json not found (skipped)"
    fi

    # Check root README.md plugin table
    local root_readme="$REPO_ROOT/README.md"
    if [[ -f "$root_readme" ]]; then
        local table_version
        table_version=$(grep -o "java-perf.*|[[:space:]]*[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*" "$root_readme" | grep -o "[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$")
        
        if [[ -z "$table_version" ]]; then
            log_warning "Root README.md plugin table entry not found (skipped)"
        elif [[ "$table_version" == "$expected_version" ]]; then
            log_success "Root README.md table: $table_version"
        else
            log_error "Root README.md table: expected $expected_version, got $table_version"
        fi
    else
        log_warning "Root README.md not found (skipped)"
    fi

    # Check CHANGELOG.md has entry for current version
    local changelog_md="$plugin_dir/CHANGELOG.md"
    if [[ -f "$changelog_md" ]]; then
        if grep -q "\[$expected_version\]" "$changelog_md"; then
            log_success "CHANGELOG.md has entry for $expected_version"
        else
            log_warning "CHANGELOG.md missing entry for $expected_version"
        fi
    else
        log_warning "CHANGELOG.md not found (skipped)"
    fi
}

# Print summary
print_summary() {
    echo ""
    echo "=========================================="
    echo "  VALIDATION SUMMARY"
    echo "=========================================="
    echo ""
    echo "Checks passed: $CHECKS_PASSED"
    echo "Checks failed: $CHECKS_FAILED"
    echo ""

    if [[ ${#ERRORS[@]} -gt 0 ]]; then
        echo -e "${RED}ERRORS:${NC}"
        for error in "${ERRORS[@]}"; do
            echo "  ✗ $error"
        done
        echo ""
        echo -e "${RED}VALIDATION FAILED${NC}"
        echo "=========================================="
        return 1
    else
        echo -e "${GREEN}ALL CHECKS PASSED${NC}"
        echo "=========================================="
        return 0
    fi
}

# Main execution
main() {
    echo "=========================================="
    echo "  Version Validation Script"
    echo "=========================================="

    local plugin_name="$1"

    if [[ -n "$plugin_name" ]]; then
        # Validate specific plugin
        if [[ ! -d "$REPO_ROOT/plugins/$plugin_name" ]]; then
            log_error "Plugin not found: $plugin_name"
            exit 1
        fi
        validate_plugin "$plugin_name"
    else
        # Validate all plugins
        for plugin_dir in "$REPO_ROOT/plugins"/*/; do
            if [[ -d "$plugin_dir" ]]; then
                local name
                name=$(basename "$plugin_dir")
                validate_plugin "$name"
            fi
        done
    fi

    print_summary
    exit $?
}

main "$@"
