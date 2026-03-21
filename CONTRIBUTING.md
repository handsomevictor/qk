# Contributing to qk

Thank you for your interest in contributing!

---

## Development Setup

```bash
# Clone and build
git clone https://github.com/<your-org>/qk
cd qk
cargo build

# Run the full test suite
cargo test --all

# Run clippy (must pass with zero warnings)
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check

# Format code
cargo fmt
```

---

## Before Every Commit

Run all three checks. CI will enforce them — save time by catching failures locally:

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test --all
```

---

## Making Changes

### Adding a new format parser

1. Add a variant to the `Format` enum in `src/detect.rs`
2. Add detection logic in `detect::sniff()`
3. Create `src/parser/<format>.rs` with `parse(input: &str) -> Result<Vec<Record>>`
4. Register it in `src/parser/mod.rs`
5. Add a fixture file in `tests/fixtures/`
6. Add integration tests in `tests/formats.rs`
7. Update `STRUCTURE.md` and `PROGRESS.md`

### Adding a new query operator (fast layer)

1. Add a variant to `FilterOp` in `src/query/fast/parser.rs`
2. Parse it in `parse_filter()`
3. Evaluate it in `eval_filter()` in `src/query/fast/eval.rs`
4. Add unit tests in `src/query/fast/eval.rs` (inline `#[cfg(test)]`)
5. Add integration tests in `tests/fast_layer.rs`

### Adding a new pipeline stage (DSL layer)

1. Add a variant to `Stage` in `src/query/dsl/ast.rs`
2. Parse it in `src/query/dsl/parser.rs`
3. Evaluate it in `apply_stage()` in `src/query/dsl/eval.rs`
4. Add tests in `tests/dsl_layer.rs`

---

## Code Style

- **No `unwrap()` in library code** — use `?` propagation or explicit `match`. `unwrap()` is only allowed in `main.rs` top level and test code.
- **Functions ≤ 40 lines** — split when longer.
- **Doc comments (`///`) on every public function and struct.**
- **Error messages must be actionable** — include field name, line number, or file path in error context.
- **English in all identifiers and comments.** Markdown docs have both English and `_CN.md` Chinese versions.

---

## Documentation

When your change affects behavior:
- Update `COMMANDS.md` and `TUTORIAL.md` (English)
- Update `COMMANDS_CN.md` and `TUTORIAL_CN.md` (Chinese)
- Update `STRUCTURE.md` if files were added or moved
- Add an entry to `PROGRESS.md`
- Add an entry to `LESSON_LEARNED.md` if a non-trivial bug was fixed

---

## Pull Request Checklist

- [ ] `cargo fmt` clean
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test --all` passes
- [ ] New behavior has tests
- [ ] `PROGRESS.md` updated
- [ ] `STRUCTURE.md` updated (if files changed)
- [ ] Docs updated (COMMANDS.md / TUTORIAL.md) if user-facing behavior changed

---

## Reporting Bugs

Open an issue with:
- The exact command you ran
- The input data (or a minimal reproducer)
- The expected output
- The actual output
- Your OS and `qk --version` output
