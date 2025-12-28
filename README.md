# Dev Skills

Developer Skills marketplace for [Claude Code](https://code.claude.com/) - providing performance diagnostics, code analysis, and more.

## Installation

```bash
# Add marketplace
/plugin marketplace add ly87ing/dev-skills

# Install java-perf plugin
/plugin install java-perf@dev-skills
```

## Available Plugins

| Plugin | Description | Version |
|--------|-------------|---------|
| [java-perf](./plugins/java-perf/) | Java performance diagnostics using AST analysis. Identifies N+1 queries, memory leaks, lock contention, and concurrency risks. | 9.5.0 |

## Version Management

Each plugin's version is managed via its `.claude-plugin/plugin.json` file (single source of truth).

To sync version across all related files for a plugin:

```bash
cd plugins/java-perf
./scripts/sync-version.sh
```

To bump version:

```bash
cd plugins/java-perf
./scripts/bump-version.sh patch  # or minor, major
```

## Plugin Development

### Directory Structure

```
dev-skills/
├── .claude-plugin/
│   └── marketplace.json      # Marketplace definition
├── plugins/
│   └── java-perf/            # Individual plugin
│       ├── .claude-plugin/plugin.json  # 🔑 Version source of truth
│       ├── skills/<name>/SKILL.md
│       ├── hooks/hooks.json
│       ├── scripts/
│       │   ├── sync-version.sh   # Sync version to all files
│       │   ├── bump-version.sh   # Bump version (major/minor/patch)
│       │   └── release.sh        # Create Git tag
│       └── rust/             # Plugin-specific code
└── scripts/
    └── validate-versions.sh  # CI version validation
```

## References

- [Agent Skills](https://code.claude.com/docs/en/skills) - How to create and distribute Skills
- [Plugin Marketplaces](https://code.claude.com/docs/en/plugin-marketplaces) - How to create and host marketplaces
- [Plugins Reference](https://code.claude.com/docs/en/plugins-reference) - Complete technical reference

## License

MIT
