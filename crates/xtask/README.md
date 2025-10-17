# xtask - Development Tools

Development utilities for Dungeon project using the [cargo-xtask pattern](https://github.com/matklad/cargo-xtask).

## Usage

Run via cargo:

```bash
cargo xtask <command>
```

Or use convenient Just recipes:

```bash
just tail-logs       # Monitor latest session logs
just clean-data      # Clean all data (with confirmation)
just clean-logs      # Clean only logs
```

## Available Commands

### `tail-logs` - Monitor Client Logs

Monitor Dungeon client logs in real-time, similar to `tail -f`.

```bash
# Monitor latest session (default)
cargo xtask tail-logs

# Monitor specific session
cargo xtask tail-logs session_1234567890

# Show more history lines before tailing
cargo xtask tail-logs -n 50
```

**Features:**
- Automatically finds latest session if not specified
- Platform-specific log directory detection (macOS/Linux/Windows)
- Configurable history lines and poll interval
- Colored output with session info

### `clean` - Clean Save Data and Logs

Clean up Dungeon's persistent data with safety confirmations.

```bash
# Clean everything (logs + save data) with confirmation
cargo xtask clean

# Clean only logs
cargo xtask clean --logs

# Clean only save data
cargo xtask clean --data

# Clean specific session logs
cargo xtask clean --logs --session session_1234567890

# Skip confirmation (dangerous!)
cargo xtask clean -y
```

**Safety:**
- Always prompts for confirmation unless `-y` flag is used
- Shows exactly what will be deleted before proceeding
- Validates session existence before attempting deletion

## Directory Conventions

Dungeon follows platform-specific directory conventions:

**Logs (cache directory):**
- macOS: `~/Library/Caches/dungeon/logs`
- Linux: `~/.cache/dungeon/logs` (or `$XDG_CACHE_HOME/dungeon/logs`)
- Windows: `%LOCALAPPDATA%\dungeon\logs`
- Fallback: `/tmp/dungeon/logs`

**Save Data (data directory):**
- macOS: `~/Library/Application Support/dungeon`
- Linux: `~/.local/share/dungeon` (or `$XDG_DATA_HOME/dungeon`)
- Windows: `%APPDATA%\dungeon`
- Fallback: `./save_data`

## Design Principles

This xtask implementation follows these principles:

1. **Simplicity**: Each command is self-contained and easy to understand
2. **Extensibility**: New commands can be added as separate modules
3. **Cross-platform**: Uses `directories` crate for platform-specific paths
4. **Safety**: Destructive operations require confirmation
5. **User-friendly**: Clear output with colors and helpful error messages

## Adding New Commands

To add a new xtask command:

1. Create new module in `src/commands/`
2. Implement command struct with `clap::Parser` derive
3. Add `execute()` method that returns `anyhow::Result<()>`
4. Export from `src/commands/mod.rs`
5. Add variant to `Command` enum in `src/main.rs`
6. Add Just recipe in workspace `justfile` for convenience

Example:

```rust
// src/commands/my_command.rs
use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct MyCommand {
    #[arg(short, long)]
    pub flag: bool,
}

impl MyCommand {
    pub fn execute(self) -> Result<()> {
        // Implementation here
        Ok(())
    }
}
```

## Dependencies

- `clap` - CLI argument parsing with derive macros
- `anyhow` - Error handling
- `directories` - Platform-specific directory detection
- `console` - Colored terminal output

All dependencies are carefully chosen to be:
- Lightweight and fast to compile
- Well-maintained and widely used
- Compatible with our target platforms
