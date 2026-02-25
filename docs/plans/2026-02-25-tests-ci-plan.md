# Tests and CI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive unit tests to all pure-logic modules and a GitHub Actions CI pipeline.

**Architecture:** Add `#[cfg(test)] mod tests` blocks to each module with tests for pure functions. Create a CI workflow that runs fmt, clippy, and test on PRs and main pushes.

**Tech Stack:** Rust, GitHub Actions, cargo test/clippy/fmt

---

### Task 1: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1: Create the CI workflow file**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Check formatting
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Tests
        run: cargo test
```

**Step 2: Verify locally**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions workflow for fmt, clippy, and test"
```

---

### Task 2: theme.rs tests

**Files:**
- Modify: `src/theme.rs`

**Tests to add:**
- `parse_hex_valid` — "#ff00aa" → Some((255, 0, 170))
- `parse_hex_no_hash` — "ff00aa" → Some((255, 0, 170))
- `parse_hex_invalid_length` — "#fff" → None
- `parse_hex_invalid_chars` — "#gggggg" → None
- `parse_hex_empty` — "" → None
- `bg_color_valid` — returns Rgb color
- `bg_color_invalid` — returns Black fallback
- `fg_color_invalid` — returns White fallback
- `palette_color_out_of_bounds` — returns Reset fallback
- `config_response_deserialize` — parse sample JSON into ConfigResponse

---

### Task 3: api.rs tests

**Files:**
- Modify: `src/api.rs`

**Tests to add:**
- `sort_order_as_str` — all three variants
- `sort_order_label` — all three variants
- `sort_order_next_cycles` — Popular→Newest→Trending→Popular
- `fetch_params_default` — verify defaults
- `urlencoding_basic` — "hello world" → "hello%20world"
- `urlencoding_special` — special chars encoded
- `urlencoding_passthrough` — alphanumerics unchanged

---

### Task 4: config.rs tests

**Files:**
- Modify: `src/config.rs`

Extract the line-filtering logic into a testable function, then test it.

**Refactor:** Extract `filter_config_lines(content: &str) -> String` as a pub(crate) function.

**Tests to add:**
- `filter_removes_background` — strips `background = #000`
- `filter_removes_palette` — strips `palette = 0=#000`
- `filter_keeps_comments` — preserves `# comment`
- `filter_keeps_empty_lines` — preserves blank lines
- `filter_keeps_non_color_keys` — preserves `font-size = 14`
- `filter_mixed_content` — full config with mix of color and non-color keys

---

### Task 5: collection.rs tests

**Files:**
- Modify: `src/collection.rs`

**Tests to add:**
- `mode_preference_labels` — all four variants
- `mode_preference_next_chain` — Dark→Light→AutoOs→AutoTime→None
- `cycle_order_serde_roundtrip` — serialize and deserialize Sequential/Shuffle
- `mode_preference_serde_roundtrip` — serialize and deserialize all variants
- `app_config_default` — verify default values (None, None, "19:00", "07:00")
- `collection_theme_serde` — serialize/deserialize CollectionTheme

---

### Task 6: app.rs tests

**Files:**
- Modify: `src/app.rs`

**Tests to add:**
- `select_next_increments` — selected goes from 0 to 1
- `select_next_clamps_at_end` — doesn't exceed themes.len()-1
- `select_prev_decrements` — selected goes from 1 to 0
- `select_prev_clamps_at_zero` — stays at 0
- `toggle_dark_filter_cycles` — None→Some(true)→Some(false)→None

---

### Task 7: Fix clippy warnings and verify

**Files:**
- Any files with clippy warnings

**Step 1: Run clippy with deny warnings**

Run: `cargo clippy -- -D warnings`

**Step 2: Fix any warnings found**

**Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass (existing 34 + new tests)

**Step 4: Run fmt check**

Run: `cargo fmt --check`

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings and verify CI readiness"
```
