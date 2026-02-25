# Tests and CI Design

## Goal

Add comprehensive unit tests across all pure-logic modules and a GitHub Actions CI pipeline for automated quality checks.

## CI Pipeline

- **Workflow file:** `.github/workflows/ci.yml`
- **Triggers:** Pull requests + pushes to main
- **Platform:** ubuntu-latest, stable Rust
- **Jobs:**
  1. `cargo fmt --check` — enforce consistent formatting
  2. `cargo clippy -- -D warnings` — catch lint issues
  3. `cargo test` — run all unit tests

## Test Coverage

Test pure logic only. No file I/O, no network calls, no terminal/OSC side effects, no process management.

### Modules to test

| Module | What to test |
|--------|-------------|
| `theme.rs` | Hex color parsing (`parse_hex_color`, `to_color`), invalid hex handling |
| `collection.rs` | `ModePreference::label()`, `ModePreference::next()`, `CycleOrder` serde, `AppConfig` defaults |
| `config.rs` | `strip_color_keys()` — correct line removal/retention from config text |
| `cycling.rs` | Eligible theme filtering by mode, sequential/shuffle index advancement |
| `preview.rs` | OSC escape string construction (correct escape codes) |
| `shell_hook.rs` | Hook snippet generation, shell detection logic |
| `api.rs` | URL/query param construction, `SortOrder` cycling, JSON response deserialization |
| `app.rs` | `cycle_mode()` state transitions, `select_next`/`select_prev` bounds, `toggle_dark_filter` cycling |

### Not testing

- UI rendering modules (browser.rs, details.rs, creator.rs, collections.rs, create_meta.rs)
- File I/O paths in config.rs and collection.rs
- Network calls in api.rs
- Daemon process management
- OSC terminal side effects (only test string construction)

### Dependencies

No new crates needed. `serde_json` is already a dependency for JSON deserialization tests.
