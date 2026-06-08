# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

**c2pa-test-tool** is a Rust CLI for creating, validating, and evaluating **C2PA (Coalition for Content Provenance and Authenticity)** test assets. It provides three core operations:

1. **Create test assets** — sign media files with C2PA manifests from test-case JSON
2. **Validate assets** — validate assets against a YAML grammar *(scaffold; grammar TBD)*
3. **Rubric evaluation** — evaluate a YAML rubric against crJSON indicators or a signed C2PA asset

## Technologies

- The C2PA specification: https://spec.c2pa.org/specifications/specifications/2.3/specs/C2PA_Specification.html
- crJSON schema: `INTERNAL/schemas/crJSON-schema.json`

## Build & Test Commands

```bash
# Build (requires c2pa-rs and profile-evaluator-rs cloned as sibling directories)
cargo build
cargo build --release

# Run tests (use --test-threads=1 for stable integration tests)
cargo test
cargo test -- --test-threads=1

# Lint & format
cargo fmt
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Architecture

### Source Layout

Single-crate package. The lib (`crtool`) exports trust utilities; the binary (`c2pa-test-tool`) contains all CLI logic.

| File | Purpose |
|------|---------|
| `src/lib.rs` | Trust settings, asset-extension helpers |
| `src/main.rs` | `clap`-based CLI, `Logger`, `run_cli()` dispatcher |
| `src/processing.rs` | C2PA manifest signing, ingredient loading, thumbnail generation, algorithm detection |
| `src/test_case.rs` | Test asset creation: reads `TestCase` JSON, resolves paths, calls processing |
| `src/batch.rs` | Batch command execution from a batch JSON file |
| `src/profile.rs` | Rubric evaluation: accepts crJSON files or signed assets (extracts crJSON via `c2pa::Reader`) |

### Critical Path Dependency

The project depends on `c2pa-rs` and `profile-evaluator-rs` via local paths (`../c2pa-rs/sdk`, `../profile-evaluator-rs`). Both repositories **must be cloned as sibling directories**.

### CLI Operating Modes

| Flag | Mode | Input → Output |
|------|------|----------------|
| `-t / --create-test <PATTERN>` | Create test asset | Test-case JSON (glob) → signed media file(s) |
| `--validate` | Validate assets *(scaffold)* | Media files → pass/fail |
| `--rubric <FILE>` | Rubric evaluation | crJSON file or signed asset + YAML rubric → YAML report (default) or JSON; asset triggers automatic crJSON extraction |
| `-b / --batch <FILE>` | Batch execution | Batch JSON → sequential command results |

### Schemas & Test Assets

- **`INTERNAL/schemas/`** — `test-case.schema.json`, `batch.schema.json`
- **`tests/fixtures/assets/raw/`** — Unsigned sample media files (Dog.jpg, Dog.png, Dog.webp)
- **`test-cases/`** — Positive and negative test cases (JSON)
- **`tests/fixtures/certs/`** — Test certificates (ed25519.pem, es256\_\*.pem)
- **`profiles/`** — YAML asset profiles for profile evaluation

## Code Quality

Formatting governed by `rustfmt.toml`: `max_width = 100`, 4-space tabs, edition 2021.

## Code Search

**Always prefer Scout (`mcp__scout__*`) over Grep/Glob.** Use `limit: 10` on all searches.

| Need | Tool |
|------|------|
| Understand how something works, find patterns, explore architecture | `investigate` |
| Exact identifier/symbol lookup | `keyword_search` |
| Jump to a symbol's definition + see references | `go_to_definition` |
| Find all imports/calls of a symbol | `find_references` |
| Blast radius of changing a symbol | `impact` |
| Pre-commit check | `detect_changes` (scope: "staged" or "all") |

## Change Safety

- **Before renaming or refactoring a symbol**, run `impact({symbol: "X", direction: "upstream"})`.
- **Before committing**, run `detect_changes({scope: "staged"})` to verify only expected symbols changed.
