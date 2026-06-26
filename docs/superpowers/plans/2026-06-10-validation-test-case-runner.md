# Validation Test Case Runner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a `-v / --validate <YAML>` CLI option that parses a validation test-case YAML, runs C2PA validation on the specified asset, and reports PASS/FAIL by comparing actual status codes against the expectations declared in the YAML.

**Architecture:** A new `src/validation.rs` module owns the YAML types and the evaluation logic. `src/main.rs` gains the `-v` short flag and a `--validate` argument that accepts a FILE path (replacing the existing scaffold bool flag). The new module reads the YAML, loads the asset via `c2pa::Reader`, extracts `validationResults` from the crJSON for each manifest, then checks each `StatusCodesExpectation` predicate and collects per-manifest PASS/FAIL results into a final report printed to stdout.

**Tech Stack:** Rust, `serde` + `serde_yaml` (already in Cargo.toml), `c2pa::Reader` + `c2pa::Context` / `Settings`, `anyhow`, `clap`.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/validation.rs` | **Create** | YAML types, expectation evaluation logic, report generation |
| `src/main.rs` | **Modify** | Replace scaffold `--validate` bool with `-v / --validate <FILE>`, wire `run_validation` |
| `tests/src/test_validation.rs` | **Create** | Integration tests covering each expectation predicate |
| `Cargo.toml` | **Modify** | Register `test_validation` test target |

---

## Task 1: Define YAML types in `src/validation.rs`

**Files:**
- Create: `src/validation.rs`

These types mirror the `validation_test.schema.json` schema exactly (camelCase keys via `serde rename`).

- [ ] **Step 1.1: Write `src/validation.rs` with the YAML data types**

```rust
// Copyright 2025 Adobe. All rights reserved.
// Licensed under Apache-2.0.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ValidationTestCase {
    pub description: String,
    pub inputs: Inputs,
    pub manifests: Vec<ManifestExpectation>,
    #[serde(rename = "validatorSpecVersions", default)]
    pub validator_spec_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Inputs {
    #[serde(rename = "assetPath")]
    pub asset_path: String,
    #[serde(rename = "claimSignerTrustListPaths")]
    pub claim_signer_trust_list_paths: Vec<String>,
    #[serde(rename = "tsaTrustListPaths")]
    pub tsa_trust_list_paths: Vec<String>,
    #[serde(rename = "validationTime", default)]
    pub validation_time: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ManifestExpectation {
    #[serde(default)]
    pub failures: StatusCodesExpectations,
    #[serde(default)]
    pub successes: StatusCodesExpectations,
    #[serde(default)]
    pub informationals: StatusCodesExpectations,
}

/// Unit struct for the schema's empty-object sentinel `{}`.
/// serde_yaml parses `isEmpty: {}` into this cleanly.
#[derive(Debug, Deserialize, Default)]
pub struct EmptyObject {}

#[derive(Debug, Deserialize, Default)]
pub struct StatusCodesExpectations {
    #[serde(rename = "isEmpty")]
    pub is_empty: Option<EmptyObject>,
    #[serde(rename = "isNotEmpty")]
    pub is_not_empty: Option<EmptyObject>,
    #[serde(rename = "containsExactly")]
    pub contains_exactly: Option<StatusCodeSet>,
    #[serde(rename = "containsAllOf")]
    pub contains_all_of: Option<StatusCodeSet>,
    #[serde(rename = "containsNoneOf")]
    pub contains_none_of: Option<StatusCodeSet>,
    #[serde(rename = "containsAnyOf")]
    pub contains_any_of: Option<Vec<StatusCodeSet>>,
}

#[derive(Debug, Deserialize)]
pub struct StatusCodeSet {
    pub codes: Vec<String>,
}
```

- [ ] **Step 1.2: Verify it compiles**

```bash
cd /Users/lrosenth/Development/c2pa-test-tool
# Add `mod validation;` to src/main.rs first (just the mod declaration, no logic yet)
cargo check 2>&1 | head -30
```

Expected: no errors (unused warnings are fine at this stage).

---

## Task 2: Implement expectation-checking helpers

**Files:**
- Modify: `src/validation.rs` (add below the types)

- [ ] **Step 2.1: Write the failing test first**

Create `tests/src/test_validation.rs`:

```rust
// Integration tests for validation expectation helpers

use c2pa_test_tool::validation::{StatusCodeSet, StatusCodesExpectations};

fn exp_is_empty() -> StatusCodesExpectations {
    StatusCodesExpectations { is_empty: Some(EmptyObject {}), ..Default::default() }
}

fn exp_contains_all_of(codes: &[&str]) -> StatusCodesExpectations {
    StatusCodesExpectations {
        contains_all_of: Some(StatusCodeSet {
            codes: codes.iter().map(|s| s.to_string()).collect(),
        }),
        ..Default::default()
    }
}

#[test]
fn test_is_empty_passes_on_empty_set() {
    let actual = vec![];
    let (pass, _) = exp_is_empty().check(&actual);
    assert!(pass);
}

#[test]
fn test_is_empty_fails_on_non_empty_set() {
    let actual = vec!["signingCredential.trusted".to_string()];
    let (pass, _) = exp_is_empty().check(&actual);
    assert!(!pass);
}

#[test]
fn test_contains_all_of_passes_when_all_present() {
    let actual = vec!["claimSignature.validated".to_string(), "signingCredential.trusted".to_string()];
    let (pass, _) = exp_contains_all_of(&["claimSignature.validated", "signingCredential.trusted"]).check(&actual);
    assert!(pass);
}

#[test]
fn test_contains_all_of_fails_when_one_missing() {
    let actual = vec!["claimSignature.validated".to_string()];
    let (pass, _) = exp_contains_all_of(&["claimSignature.validated", "signingCredential.trusted"]).check(&actual);
    assert!(!pass);
}
```

Add to `Cargo.toml`:
```toml
[[test]]
name = "test_validation"
path = "tests/src/test_validation.rs"
```

Also expose `validation` module from `src/lib.rs`:
```rust
pub mod validation;
```

- [ ] **Step 2.2: Run test to confirm it fails (module not yet impl)**

```bash
cargo test --test test_validation 2>&1 | head -20
```

Expected: compile error — `check` method not found.

- [ ] **Step 2.3: Implement `StatusCodesExpectations::check`**

Add to `src/validation.rs`:

```rust
impl StatusCodesExpectations {
    /// Returns (passed, Vec<failure_reason>).
    pub fn check(&self, actual: &[String]) -> (bool, Vec<String>) {
        let mut failures = Vec::new();
        let actual_set: std::collections::HashSet<&str> =
            actual.iter().map(String::as_str).collect();

        if self.is_empty.is_some() && !actual.is_empty() {
            failures.push(format!("expected empty, got: {:?}", actual));
        }

        if self.is_not_empty.is_some() && actual.is_empty() {
            failures.push("expected non-empty, got empty set".to_string());
        }

        if let Some(set) = &self.contains_exactly {
            let expected_set: std::collections::HashSet<&str> =
                set.codes.iter().map(String::as_str).collect();
            if actual_set != expected_set {
                failures.push(format!(
                    "expected exactly {:?}, got {:?}",
                    set.codes, actual
                ));
            }
        }

        if let Some(set) = &self.contains_all_of {
            let missing: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| !actual_set.contains(c))
                .collect();
            if !missing.is_empty() {
                failures.push(format!("missing required codes: {:?}", missing));
            }
        }

        if let Some(set) = &self.contains_none_of {
            let found: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| actual_set.contains(c))
                .collect();
            if !found.is_empty() {
                failures.push(format!("unexpected codes present: {:?}", found));
            }
        }

        if let Some(any_of_sets) = &self.contains_any_of {
            for (i, set) in any_of_sets.iter().enumerate() {
                let has_any = set.codes.iter().any(|c| actual_set.contains(c.as_str()));
                if !has_any {
                    failures.push(format!(
                        "containsAnyOf[{}]: none of {:?} found in actual",
                        i, set.codes
                    ));
                }
            }
        }

        (failures.is_empty(), failures)
    }
}
```

- [ ] **Step 2.4: Run test to verify it passes**

```bash
cargo test --test test_validation -- --test-threads=1 2>&1 | tail -20
```

Expected: 4 tests pass.

- [ ] **Step 2.5: Commit**

```bash
git add src/validation.rs tests/src/test_validation.rs Cargo.toml src/lib.rs
git commit -m "feat: add validation test-case YAML types and expectation checker"
```

---

## Task 3: Implement `run_validation` — load asset and extract status codes

**Files:**
- Modify: `src/validation.rs`

The key insight: crJSON manifests are in `validationResults.success/informational/failure` as objects with a `code` string field. We parse the crJSON as `serde_json::Value` to extract them, avoiding a full crJSON struct dependency.

- [ ] **Step 3.1: Write failing test for asset loading**

Add to `tests/src/test_validation.rs`:

```rust
use std::path::Path;
use c2pa_test_tool::validation::run_validation;

#[test]
fn test_run_validation_png_valid() {
    let yaml_path = Path::new("tests/validation/png_valid.yaml");
    let result = run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    let report = result.unwrap();
    assert!(report.overall_pass, "Expected PASS but got FAIL:\n{}", report.summary());
}
```

Run to confirm it fails:

```bash
cargo test --test test_validation test_run_validation_png_valid 2>&1 | head -20
```

Expected: compile error — `run_validation` not found.

- [ ] **Step 3.2: Implement `run_validation` and `ValidationReport`**

Add to `src/validation.rs`:

```rust
use anyhow::{Context, Result};
use std::path::Path;

pub struct ManifestResult {
    pub index: usize,
    pub pass: bool,
    pub reasons: Vec<String>,
    pub actual_successes: Vec<String>,
    pub actual_failures: Vec<String>,
    pub actual_informationals: Vec<String>,
}

pub struct ValidationReport {
    pub description: String,
    pub overall_pass: bool,
    pub manifests: Vec<ManifestResult>,
    pub validation_time_ignored: bool,
}

impl ValidationReport {
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Description: {}", self.description));
        lines.push(format!(
            "Overall: {}",
            if self.overall_pass { "PASS" } else { "FAIL" }
        ));
        for m in &self.manifests {
            lines.push(format!(
                "  Manifest[{}]: {}",
                m.index,
                if m.pass { "PASS" } else { "FAIL" }
            ));
            for r in &m.reasons {
                lines.push(format!("    - {}", r));
            }
        }
        lines.join("\n")
    }
}

/// Load and evaluate a validation test-case YAML file.
/// Paths inside the YAML are resolved relative to the YAML file's parent directory.
pub fn run_validation(yaml_path: &Path) -> Result<ValidationReport> {
    let yaml_dir = yaml_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let yaml_str = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("Failed to read validation YAML: {}", yaml_path.display()))?;

    let test_case: ValidationTestCase = serde_yaml::from_str(&yaml_str)
        .with_context(|| format!("Failed to parse validation YAML: {}", yaml_path.display()))?;

    let asset_path = yaml_dir.join(&test_case.inputs.asset_path);

    // Build trust anchors PEM string by concatenating all files
    let mut trust_pem = String::new();
    for pem_path in &test_case.inputs.claim_signer_trust_list_paths {
        let full = yaml_dir.join(pem_path);
        let pem = std::fs::read_to_string(&full)
            .with_context(|| format!("Failed to read trust list PEM: {}", full.display()))?;
        trust_pem.push_str(&pem);
    }

    // Merge TSA trust list PEMs into the same bundle (c2pa-rs has no separate TSA anchor setting)
    for pem_path in &test_case.inputs.tsa_trust_list_paths {
        let full = yaml_dir.join(pem_path);
        let pem = std::fs::read_to_string(&full)
            .with_context(|| format!("Failed to read TSA trust list PEM: {}", full.display()))?;
        trust_pem.push_str(&pem);
    }

    // Build trust settings using the existing crtool helper (reuses established pattern)
    if trust_pem.trim().is_empty() {
        anyhow::bail!("No trust anchors provided (claimSignerTrustListPaths and tsaTrustListPaths are both empty)");
    }
    let settings = crtool::build_trust_settings(trust_pem.trim(), None, None)
        .context("Failed to configure trust anchors")?;

    // Context::new().with_settings accepts a Settings value via IntoSettings
    let context = c2pa::Context::new()
        .with_settings(settings)
        .context("Failed to create c2pa context with trust settings")?;

    let reader = c2pa::Reader::from_context(context)
        .with_file(&asset_path)
        .with_context(|| format!("Failed to read asset: {}", asset_path.display()))?;

    // Parse the crJSON to extract per-manifest validation results
    let crjson_value: serde_json::Value = serde_json::from_str(&reader.crjson())
        .context("Failed to parse crJSON output")?;

    let manifests_json = crjson_value["manifests"]
        .as_array()
        .context("crJSON has no 'manifests' array")?;

    // Special case: empty manifests array means "expect no C2PA manifests in asset"
    if test_case.manifests.is_empty() {
        let overall_pass = manifests_json.is_empty();
        return Ok(ValidationReport {
            description: test_case.description,
            overall_pass,
            manifests: if overall_pass {
                vec![]
            } else {
                vec![ManifestResult {
                    index: 0,
                    pass: false,
                    reasons: vec![format!(
                        "expected no manifests but asset contains {}",
                        manifests_json.len()
                    )],
                    actual_successes: vec![],
                    actual_failures: vec![],
                    actual_informationals: vec![],
                }]
            },
            validation_time_ignored: test_case.inputs.validation_time.is_some(),
        });
    }

    let mut manifest_results = Vec::new();
    let mut overall_pass = true;

    for (i, expected) in test_case.manifests.iter().enumerate() {
        let manifest_json = manifests_json.get(i).with_context(|| {
            format!(
                "crJSON has {} manifest(s) but test case expects at least {}",
                manifests_json.len(),
                i + 1
            )
        })?;

        let vr = &manifest_json["validationResults"];

        let actual_successes = extract_codes(&vr["success"]);
        let actual_failures = extract_codes(&vr["failure"]);
        let actual_informationals = extract_codes(&vr["informational"]);

        let mut reasons = Vec::new();

        let (s_pass, s_reasons) = expected.successes.check(&actual_successes);
        for r in s_reasons {
            reasons.push(format!("[successes] {}", r));
        }

        let (f_pass, f_reasons) = expected.failures.check(&actual_failures);
        for r in f_reasons {
            reasons.push(format!("[failures] {}", r));
        }

        let (i_pass, i_reasons) = expected.informationals.check(&actual_informationals);
        for r in i_reasons {
            reasons.push(format!("[informationals] {}", r));
        }

        let pass = s_pass && f_pass && i_pass;
        if !pass {
            overall_pass = false;
        }
        manifest_results.push(ManifestResult {
            index: i,
            pass,
            reasons,
            actual_successes,
            actual_failures,
            actual_informationals,
        });
    }

    Ok(ValidationReport {
        description: test_case.description,
        overall_pass,
        manifests: manifest_results,
        validation_time_ignored: test_case.inputs.validation_time.is_some(),
    })
}

fn extract_codes(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry["code"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
```

- [ ] **Step 3.3: Run the test**

```bash
cargo test --test test_validation test_run_validation_png_valid -- --test-threads=1 2>&1 | tail -30
```

Expected: PASS. If the trust settings API differs, adjust the `with_value` key names by checking `c2pa-rs` Settings docs/tests.

> **Note on `validationTime`:** `c2pa-rs` does not currently expose a way to override the validation clock via settings. Log a warning if `validationTime` is set and proceed without it. Track this as a known limitation in the report when the field is present.

- [ ] **Step 3.4: Add tests for mp3 and mp4 YAML files**

Add to `tests/src/test_validation.rs`:

```rust
#[test]
fn test_run_validation_mp3_valid() {
    let yaml_path = Path::new("tests/validation/mp3_valid.yaml");
    let result = run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    assert!(result.unwrap().overall_pass);
}

#[test]
fn test_run_validation_mp4_valid() {
    let yaml_path = Path::new("tests/validation/mp4_valid.yaml");
    let result = run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    assert!(result.unwrap().overall_pass);
}
```

```bash
cargo test --test test_validation -- --test-threads=1 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 3.5: Commit**

```bash
git add src/validation.rs tests/src/test_validation.rs
git commit -m "feat: implement run_validation — asset loading and status code extraction"
```

---

## Task 4: Wire `-v / --validate <FILE>` into the CLI

**Files:**
- Modify: `src/main.rs`

Replace the existing `--validate` bool + `--grammar` with a single `-v / --validate <FILE>` that takes the YAML path. No input files are needed since the asset path is embedded in the YAML.

- [ ] **Step 4.1: Update `Cli` struct in `src/main.rs`**

Remove:
```rust
/// Validate input asset(s) against a YAML asset grammar
/// (schema TBD; currently scaffolded — always reports success)
#[arg(long, default_value = "false")]
validate: bool,

/// Path to the YAML asset grammar for validation (required with --validate)
#[arg(long, value_name = "FILE")]
grammar: Option<PathBuf>,
```

Add:
```rust
/// Path to a validation test-case YAML file (validation_test.schema.json).
/// Resolves the asset path embedded in the YAML and evaluates expected vs actual
/// C2PA validation status codes, reporting PASS or FAIL.
#[arg(short = 'v', long = "validate", value_name = "FILE")]
validate: Option<PathBuf>,
```

- [ ] **Step 4.2: Add `use validation::run_validation;` import and `mod validation;`**

At the top of `src/main.rs` add:
```rust
mod validation;
use validation::run_validation;
```

- [ ] **Step 4.3: Replace the scaffold `--validate` branch in `run_cli`**

Remove the old scaffold block:
```rust
// ── Validate mode (scaffold) ──────────────────────────────────────────────
if cli.validate {
    ...
    return Ok(());
}
```

**CRITICAL:** Place the new `--validate` block **immediately after the `--create-test` early return and BEFORE the `input.is_empty()` guard** (around line 237 of `src/main.rs`). Running `-v foo.yaml` passes no positional input files, so if the block is placed after the guard it will always fail with "No input files specified".

Insert between the `--create-test` return and the `if cli.input.is_empty()` check:
```rust
// ── Validate mode ────────────────────────────────────────────────────────
if let Some(yaml_path) = &cli.validate {
    logger.info(&format!("=== Validation Test Case: {} ===", yaml_path.display()));
    match run_validation(yaml_path) {
        Ok(report) => {
            logger.info(&report.summary());
            if !report.overall_pass {
                anyhow::bail!("Validation FAILED");
            }
        }
        Err(e) => {
            anyhow::bail!("Validation error: {:#}", e);
        }
    }
    return Ok(());
}
```

Also update the error message in the final `anyhow::bail!` at the bottom of `run_cli` to replace `--validate to validate assets` with `--validate FILE to run a validation test case`.

- [ ] **Step 4.5: Build and smoke-test**

```bash
cargo build 2>&1 | tail -20
./target/debug/c2pa-test-tool -v tests/validation/png_valid.yaml
```

Expected output:
```
=== Validation Test Case: tests/validation/png_valid.yaml ===
Description: PNG with valid C2PA manifest
Overall: PASS
  Manifest[0]: PASS
```

- [ ] **Step 4.6: Commit**

```bash
git add src/main.rs src/validation.rs
git commit -m "feat: wire -v/--validate <FILE> CLI option for validation test cases"
```

---

## Task 5: Report formatting — clear PASS/FAIL output

**Files:**
- Modify: `src/validation.rs`

The `summary()` from Task 3 is functional but plain. Polish it: add an emoji banner, show actual codes on failure, and include the `validationTime` warning line. The `ManifestResult` and `ValidationReport` structs already have all needed fields from Task 3 — this task only changes `summary()`.

- [ ] **Step 5.1: Update `summary()` to print actual codes on failure**

```rust
pub fn summary(&self) -> String {
    let banner = if self.overall_pass {
        "✅ PASS"
    } else {
        "❌ FAIL"
    };
    let mut lines = vec![
        format!("{} — {}", banner, self.description),
    ];
    for m in &self.manifests {
        let label = if m.pass { "✅ PASS" } else { "❌ FAIL" };
        lines.push(format!("  Manifest[{}]: {}", m.index, label));
        if !m.pass {
            for r in &m.reasons {
                lines.push(format!("    ✗ {}", r));
            }
            // Show actual codes to help diagnose
            if !m.actual_successes.is_empty() {
                lines.push(format!("    actual successes:      {:?}", m.actual_successes));
            }
            if !m.actual_failures.is_empty() {
                lines.push(format!("    actual failures:       {:?}", m.actual_failures));
            }
            if !m.actual_informationals.is_empty() {
                lines.push(format!(
                    "    actual informationals: {:?}",
                    m.actual_informationals
                ));
            }
        }
    }
    lines.join("\n")
}
```

- [ ] **Step 5.3: Run all tests**

```bash
cargo test -- --test-threads=1 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5.4: Commit**

```bash
git add src/validation.rs
git commit -m "feat: improve validation report with PASS/FAIL banner and actual codes on failure"
```

---

## Task 6: Handle `validationTime` limitation gracefully

**Files:**
- Modify: `src/validation.rs`

The `validationTime` field in the YAML test case schema specifies a fixed clock time for validation (RFC 3339). `c2pa-rs` doesn't expose a clock override in its public settings API as of the current version.

- [ ] **Step 6.1: Add a warning when `validationTime` is set**

In `run_validation`, after parsing the YAML, add:

```rust
if test_case.inputs.validation_time.is_some() {
    eprintln!(
        "Warning: 'validationTime' is specified in the test case but is not yet \
         supported by c2pa-rs. Validation will use the current system clock."
    );
}
```

The `validation_time_ignored` flag is already part of `ValidationReport` (added in Task 3) and the `summary()` already renders it (Task 5). This step just ensures the `eprintln!` warning is present.

- [ ] **Step 6.2: Run all tests**

```bash
cargo test -- --test-threads=1 2>&1 | tail -20
```

Expected: all pass (the `eprintln!` warning will appear in test output, that's fine).

- [ ] **Step 6.3: Commit**

```bash
git add src/validation.rs
git commit -m "feat: warn when validationTime is present but not supported"
```

---

## Task 7: Final polish — lint, fmt, integration test

- [ ] **Step 7.1: Format and lint**

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -30
```

Fix any clippy warnings in `src/validation.rs`.

- [ ] **Step 7.2: Run full test suite**

```bash
cargo test -- --test-threads=1 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 7.3: Final commit**

```bash
git add -p
git commit -m "chore: fmt and clippy fixes for validation module"
```

---

## Known Limitations (document in code comments)

1. **`validationTime`** — `c2pa-rs` does not expose a public API to override the validation clock. Tests that depend on this field will pass only if the real-time clock satisfies the cert validity window. A warning is emitted.
2. **`tsaTrustListPaths`** — TSA trust list anchors are merged into the same `trust.trust_anchors` PEM bundle as claim-signer anchors (`c2pa-rs` has no separate TSA anchor setting). Works correctly when both lists share the same root CA, which is the case for all current test fixtures.
3. **Manifest count mismatch** — if the asset has fewer manifests than the test case expects, the run fails with a clear error message. The reverse (more manifests than expected) does not fail — only the explicitly expected manifests are checked.
