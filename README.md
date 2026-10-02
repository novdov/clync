# clync

CLI tool for syncing `~/.claude` config files with a GitHub repository.

## Prerequisites

- [GitHub CLI](https://cli.github.com/) installed and authenticated (`gh auth login`)
- A private GitHub repository for syncing

## Setup

```sh
# Set remote repository
clync config repo owner/repo

# Add files to sync
clync config whitelist add settings.json
clync config whitelist add CLAUDE.md
clync config whitelist add "commands/**/*.md"
```

Config is stored at `~/.clync/config.toml`.

## Whitelist format

Relative paths from `~/.claude` with glob pattern support.

- `settings.json` - single file
- `commands/**/*.md` - all .md files in subdirectories
- `**/*.json` - all .json files recursively

```sh
clync config whitelist list              # list entries
clync config whitelist add <path>        # add entry
clync config whitelist remove <path>     # remove entry
```

## Exclude format

Paths excluded from sync even if they match the whitelist. Supports glob patterns, and a plain directory path excludes everything under it.

- `skills/private` - everything under `skills/private`
- `skills/private/**` - same as above, using glob
- `commands/git/commit.md` - single file

```sh
clync config whitelist exclude list            # list entries
clync config whitelist exclude add <path>      # add entry
clync config whitelist exclude remove <path>   # remove entry
```

## Usage

```sh
# Push local changes to remote
clync push

# Pull remote changes to local
clync pull

# Show diff between local and remote
clync diff

# Show sync status summary
clync status
```

Both `push` and `pull` support `--dry-run` (preview without changes) and `--force` (skip confirmation).