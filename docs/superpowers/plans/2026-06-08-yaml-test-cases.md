# YAML Test Cases Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate all 41 test case files from JSON to YAML (manifest field stays as a raw JSON block scalar), update the Rust parser, add integration tests that run every test case across five media formats, and update all documentation.

**Architecture:** Clean break — no dual-format support. `serde_yaml` moves to regular dependencies. `TestCase` gains a top-level `alg: Option<String>` field and `manifest` becomes a `String`. Integration tests call the CLI binary as a subprocess, rotating through five input asset formats. A one-time Python script performs the bulk JSON-to-YAML conversion.

**Tech Stack:** Rust, `serde_yaml = "0.9"`, `glob = "0.3"`, Python 3 (one-time conversion script), `cargo test`

**Spec:** `docs/superpowers/specs/2026-06-08-yaml-test-cases-design.md`

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Task 2: move `serde_yaml` to `[dependencies]`; Task 6: add `[[test]]` for `test_creation` |
| Modify | `src/test_case.rs` | `TestCase.manifest: String`, add `alg: Option<String>`, switch to `serde_yaml` |
| Modify | `src/main.rs` | Update `--create-test` help text |
| Create | `tests/src/test_creation.rs` | Integration tests: glob all YAML files, run CLI subprocess, assert output |
| Create | `scripts/convert_to_yaml.py` | One-time conversion script (delete after use) |
| Delete | `testfiles-creation/*.json` | Replaced by YAML equivalents |
| Create | `testfiles-creation/*.yaml` | 41 converted test case files |
| Modify | `schemas/test-case.schema.json` | Add `alg`; change `manifest` to `type: string` |
| Modify | `schemas/test-case-description.md` | Full rewrite for YAML format |
| Modify | `README.md` | Update examples and format section |
| Modify | `tests/README.md` | Add `test_creation` suite |

---

## Task 1: Create Feature Branch

**Files:** none

- [ ] **Step 1: Create and check out branch**

```bash
git checkout -b feature/yaml-test-cases
```

Expected: `Switched to a new branch 'feature/yaml-test-cases'`

---

## Task 2: Move serde_yaml to Regular Dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Move `serde_yaml` from `[dev-dependencies]` to `[dependencies]`**

In `Cargo.toml`, remove `serde_yaml = "0.9"` from `[dev-dependencies]` and add it to `[dependencies]`:

```toml
[dependencies]
# ... existing entries ...
serde_yaml = "0.9"
```

Do **not** add the `[[test]]` entry yet — that happens in Task 6 when the test file is created. Adding it now would cause `cargo test` to fail (Cargo errors on a `[[test]]` entry that points to a non-existent file).

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1 | tail -5
```

Expected: compiles without errors.

---

## Task 3: Write Failing Unit Tests for YAML Parsing

**Files:**
- Modify: `src/test_case.rs`

These tests validate the new YAML parsing behavior. They must fail now (before the struct changes) and pass after Task 4.

- [ ] **Step 1: Add unit tests at the bottom of `src/test_case.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_YAML: &str = r#"
testId: test.minimal
signingCert: certs/test.pem
manifest: |
  {"claim_generator_info": [{"name": "test"}], "assertions": []}
"#;

    const YAML_WITH_ALG: &str = r#"
testId: test.with-alg
alg: Es256
signingCert: certs/test.pem
manifest: |
  {"claim_generator_info": [{"name": "test"}], "assertions": []}
"#;

    #[test]
    fn test_parse_yaml_minimal() {
        let tc: TestCase = serde_yaml::from_str(MINIMAL_YAML).expect("failed to parse YAML");
        assert_eq!(tc.test_id, "test.minimal");
        assert_eq!(tc.signing_cert, "certs/test.pem");
        assert!(tc.manifest.trim().starts_with('{'));
        assert!(tc.alg.is_none());
    }

    #[test]
    fn test_parse_yaml_with_alg() {
        let tc: TestCase = serde_yaml::from_str(YAML_WITH_ALG).expect("failed to parse YAML");
        assert_eq!(tc.alg.as_deref(), Some("Es256"));
    }

    #[test]
    fn test_manifest_is_valid_json_after_trim() {
        let tc: TestCase = serde_yaml::from_str(MINIMAL_YAML).expect("failed to parse YAML");
        let trimmed = tc.manifest.trim();
        serde_json::from_str::<serde_json::Value>(trimmed)
            .expect("trimmed manifest must be valid JSON");
    }
}
```

- [ ] **Step 2: Run the tests — they must fail**

```bash
cargo test test_parse_yaml 2>&1 | tail -20
```

Expected: compile error or panic about `manifest` type mismatch — confirms tests are actually testing the new behavior.

---

## Task 4: Update `TestCase` Struct and Parser

**Files:**
- Modify: `src/test_case.rs`

- [ ] **Step 1: Update the `TestCase` struct**

Replace the existing struct:

```rust
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub test_id: String,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub input_asset: Option<String>,
    pub manifest: serde_json::Value,
    pub signing_cert: String,
    pub signing_key: Option<String>,
    pub tsa_url: Option<String>,
    #[allow(dead_code)]
    pub expected_results: Option<serde_json::Value>,
}
```

With:

```rust
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub test_id: String,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub input_asset: Option<String>,
    pub alg: Option<String>,
    pub manifest: String,
    pub signing_cert: String,
    pub signing_key: Option<String>,
    pub tsa_url: Option<String>,
    #[allow(dead_code)]
    pub expected_results: Option<serde_json::Value>,
}
```

- [ ] **Step 2: Update `handle_create_test` to use `serde_yaml` and the new fields**

Replace the function body of `handle_create_test` as follows. The changes are:
1. `fs::read_to_string` error message → "YAML file"
2. `serde_json::from_str` → `serde_yaml::from_str`, error message → "YAML"
3. `manifest_json` is now `test_case.manifest.trim().to_owned()` (strips trailing `\n` from YAML block scalar)
4. `alg` detection reads `test_case.alg` first; falls back to cert auto-detection

```rust
pub fn handle_create_test(
    test_case_path: &Path,
    input_override: Option<&Path>,
    output: &Path,
) -> Result<()> {
    println!(
        "=== Creating test asset from test case: {:?} ===",
        test_case_path
    );

    let yaml_str =
        fs::read_to_string(test_case_path).context("Failed to read test case YAML file")?;
    let test_case: TestCase = serde_yaml::from_str(&yaml_str)
        .context("Failed to parse test case YAML (does it match the test case schema?)")?;

    let base_dir = test_case_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let input_asset = if let Some(override_path) = input_override {
        override_path.to_path_buf()
    } else if let Some(ref asset) = test_case.input_asset {
        base_dir.join(asset)
    } else {
        anyhow::bail!(
            "No input asset specified: the test case YAML does not include 'inputAsset' and \
            no input file was provided on the command line."
        )
    };
    let cert = base_dir.join(&test_case.signing_cert);
    let key = base_dir.join(
        test_case
            .signing_key
            .as_deref()
            .unwrap_or(&test_case.signing_cert),
    );

    // Strip trailing newline added by YAML block scalar
    let manifest_json = test_case.manifest.trim().to_owned();

    let signing_alg = if let Some(ref alg_str) = test_case.alg {
        parse_signing_algorithm(alg_str)?
    } else {
        println!("No alg in test case — auto-detecting signing algorithm from certificate...");
        let detected = detect_signing_algorithm(&cert)?;
        println!("  Detected: {:?}", detected);
        detected
    };

    println!("  Test ID:   {}", test_case.test_id);
    if let Some(title) = &test_case.title {
        println!("  Title:     {}", title);
    }
    println!("  Input:     {:?}", input_asset);
    println!("  Cert:      {:?}", cert);
    println!("  Algorithm: {:?}", signing_alg);
    if let Some(tsa) = &test_case.tsa_url {
        println!("  TSA URL:   {}", tsa);
    }

    let config = ProcessingConfig {
        manifest_json: &manifest_json,
        ingredients_base_dir: &base_dir,
        cert: &cert,
        key: &key,
        signing_alg,
        tsa_url: test_case.tsa_url.clone(),
        allow_self_signed: true,
    };

    process_single_file(&input_asset, output, &config)?;

    println!("\n✓ Test asset created successfully");
    println!("  Output: {:?}", output);
    Ok(())
}
```

- [ ] **Step 3: Run the unit tests — they must now pass**

```bash
cargo test test_parse_yaml 2>&1 | tail -20
```

Expected: all 3 unit tests pass.

- [ ] **Step 4: Run the two existing test suites (not test_creation — that file doesn't exist yet)**

```bash
cargo test --test integration_tests --test test_profile_evaluation -- --test-threads=1 2>&1 | tail -30
```

Expected: all tests in `integration_tests` and `test_profile_evaluation` pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/test_case.rs
git commit -m "feat: migrate TestCase parser from JSON to YAML

- Move serde_yaml to regular dependencies (needed in binary, not just tests)
- Change manifest field from serde_json::Value to String
- Add top-level alg: Option<String> field to TestCase struct
- Use serde_yaml::from_str for parsing; trim trailing newline from manifest block scalar
- Alg detection reads test_case.alg; falls back to cert auto-detection"
```

---

## Task 5: Update `src/main.rs` Help Text

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update the `--create-test` argument description**

Find this line in the `Cli` struct:

```rust
/// Path or glob pattern for test case JSON file(s) (C2PA test case schema).
/// Supports glob patterns (e.g., "test-cases/positive/tc-*.json", "test-cases/**/*.json").
```

Replace with:

```rust
/// Path or glob pattern for test case YAML file(s) (C2PA test case schema).
/// Supports glob patterns (e.g., "test-cases/positive/tc-*.yaml", "test-cases/**/*.yaml").
```

- [ ] **Step 2: Commit**

```bash
git add src/main.rs
git commit -m "docs: update --create-test help text from JSON to YAML"
```

---

## Task 6: Write the `test_creation` Integration Test

**Files:**
- Create: `tests/src/test_creation.rs`
- Modify: `Cargo.toml`

This test will fail until the YAML files exist (Task 7). Write it now so the skeleton is in place.

- [ ] **Step 1: Add the `[[test]]` entry to `Cargo.toml`**

Append to the end of `Cargo.toml` (after the existing `[[test]]` entries):

```toml
[[test]]
name = "test_creation"
path = "tests/src/test_creation.rs"
```

- [ ] **Step 2: Create `tests/src/test_creation.rs`**

```rust
/*
Copyright 2025 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::{cli_binary_path, output_dir};

/// Input assets rotated across test cases to exercise multiple media formats.
/// Selection is index-based (deterministic) so runs are reproducible.
const INPUT_ASSETS: &[(&str, &str)] = &[
    ("tests/fixtures/assets/raw/Dog.jpg", "jpg"),
    ("tests/fixtures/assets/raw/Dog.png", "png"),
    ("tests/fixtures/assets/raw/Dog.webp", "webp"),
    ("tests/fixtures/assets/raw/Ghostscript_tiger.svg", "svg"),
    ("tests/fixtures/assets/raw/video1_with_thumb.mp4", "mp4"),
];

/// Run every YAML test case in testfiles-creation/ through the CLI and assert
/// that a non-empty output asset is produced.
#[test]
fn test_all_testfiles_creation() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let testfiles_dir = manifest_dir.join("testfiles-creation");
    let out_dir = output_dir();
    let binary = cli_binary_path();

    let mut yaml_files: Vec<PathBuf> = std::fs::read_dir(&testfiles_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    yaml_files.sort();

    assert!(
        !yaml_files.is_empty(),
        "No YAML test case files found in testfiles-creation/ — was the conversion run?"
    );

    let mut failures = Vec::new();

    for (i, yaml_path) in yaml_files.iter().enumerate() {
        let (asset_rel, ext) = INPUT_ASSETS[i % INPUT_ASSETS.len()];
        let input_path = manifest_dir.join(asset_rel);
        let stem = yaml_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_path = out_dir.join(format!("tc-{}-{}.{}", i, stem, ext));

        let result = Command::new(&binary)
            .args([
                "--create-test",
                yaml_path.to_str().unwrap(),
                input_path.to_str().unwrap(),
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()?;

        if !result.status.success() {
            failures.push(format!(
                "  FAIL {:?}\n    stdout: {}\n    stderr: {}",
                yaml_path.file_name().unwrap(),
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            ));
            continue;
        }

        if !output_path.exists() {
            failures.push(format!(
                "  FAIL {:?} — output file not created",
                yaml_path.file_name().unwrap()
            ));
            continue;
        }

        if output_path.metadata()?.len() == 0 {
            failures.push(format!(
                "  FAIL {:?} — output file is empty",
                yaml_path.file_name().unwrap()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} test case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );

    println!(
        "✓ All {} test case files processed successfully",
        yaml_files.len()
    );
    Ok(())
}
```

- [ ] **Step 3: Verify the test compiles (it will fail to find YAML files at runtime, that's fine)**

```bash
cargo test --test test_creation -- --list 2>&1 | tail -10
```

Expected: lists `test_all_testfiles_creation` — means it compiled. If it panics "No YAML files found", that's expected too.

---

## Task 7: Convert JSON Test Case Files to YAML

**Files:**
- Create: `scripts/convert_to_yaml.py` (temporary)
- Create: `testfiles-creation/*.yaml` (41 files)
- Delete: `testfiles-creation/*.json` (41 files)

- [ ] **Step 1: Create the conversion script**

Create `scripts/convert_to_yaml.py`:

```python
#!/usr/bin/env python3
"""One-time script: convert testfiles-creation/*.json to *.yaml.

The manifest field is preserved as a raw JSON block scalar.
All other fields are written as native YAML scalars.
Run from the repository root: python3 scripts/convert_to_yaml.py
"""
import json
import os

FIELD_ORDER = [
    "testId", "title", "description", "alg", "inputAsset",
    "signingCert", "signingKey", "tsaUrl",
]


def needs_quoting(s: str) -> bool:
    """Return True if a bare YAML scalar would be ambiguous."""
    specials = set(':#{}[]|>&*!,\'"?')
    return (
        s.startswith("http")
        or any(c in s for c in specials)
        or s.lower() in ("true", "false", "null", "yes", "no")
    )


def scalar(value) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    s = str(value)
    if needs_quoting(s):
        escaped = s.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return s


def convert(json_path: str) -> None:
    with open(json_path) as f:
        data = json.load(f)

    manifest = data.pop("manifest")
    manifest_json = json.dumps(manifest, indent=2)

    lines = []
    # Write fields in preferred order
    for key in FIELD_ORDER:
        if key in data:
            lines.append(f"{key}: {scalar(data[key])}")
    # Any remaining fields not in FIELD_ORDER
    for key, val in data.items():
        if key not in FIELD_ORDER:
            lines.append(f"{key}: {scalar(val)}")
    # Manifest as indented JSON block scalar
    lines.append("manifest: |")
    for line in manifest_json.split("\n"):
        lines.append("  " + line if line.strip() else "")

    yaml_path = json_path[:-5] + ".yaml"
    with open(yaml_path, "w") as f:
        f.write("\n".join(lines) + "\n")

    os.remove(json_path)
    print(f"  {os.path.basename(json_path)} → {os.path.basename(yaml_path)}")


folder = "testfiles-creation"
count = 0
for fname in sorted(os.listdir(folder)):
    if fname.endswith(".json"):
        convert(os.path.join(folder, fname))
        count += 1

print(f"\nConverted {count} files.")
```

**Note on `alg`:** None of the existing JSON files carry a top-level `alg` value in the manifest envelope (the two occurrences of `"alg"` in the JSON are inside assertion data payloads). The converted YAML files will therefore have no `alg` field. This is intentional — `alg` is optional and the tool auto-detects the algorithm from the certificate at runtime.

- [ ] **Step 2: Run the conversion script from the repo root**

```bash
python3 scripts/convert_to_yaml.py
```

Expected output:
```
  n-actions-created-gathered.json → n-actions-created-gathered.yaml
  n-actions-created-nodst.json → n-actions-created-nodst.yaml
  ...
Converted 41 files.
```

- [ ] **Step 3: Spot-check a converted file**

```bash
cat testfiles-creation/p-actions-created.yaml
```

Expected: YAML envelope with camelCase keys, `manifest: |` followed by indented JSON.

- [ ] **Step 4: Verify no JSON files remain**

```bash
ls testfiles-creation/*.json 2>&1
```

Expected: `ls: ... No such file or directory`

- [ ] **Step 5: Run the test_creation suite**

```bash
cargo test --test test_creation -- --test-threads=1 2>&1 | tail -20
```

Expected: `test test_all_testfiles_creation ... ok`

If any test cases fail, examine the error output and fix the converted YAML for those files.

- [ ] **Step 6: Run the full test suite**

```bash
cargo test -- --test-threads=1 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add testfiles-creation/ tests/src/test_creation.rs scripts/convert_to_yaml.py
git commit -m "feat: convert test cases from JSON to YAML; add creation integration tests

- Convert all 41 test case files in testfiles-creation/ from JSON to YAML
- manifest field preserved as raw JSON block scalar (|)
- Add tests/src/test_creation.rs: runs every YAML test case through the CLI
  rotating across 5 input formats (jpg, png, webp, svg, mp4)
- Add scripts/convert_to_yaml.py (one-time conversion utility)"
```

---

## Task 8: Update `schemas/test-case.schema.json`

**Files:**
- Modify: `schemas/test-case.schema.json`

- [ ] **Step 1: Update the schema**

Make the following changes to `schemas/test-case.schema.json`:

1. Add `alg` as an optional top-level property (after `inputAsset`):
```json
"alg": {
    "type": "string",
    "description": "The signing algorithm. Valid values: Es256, Es384, Es512, Ps256, Ed25519. If omitted, the implementation auto-detects the algorithm from the signing certificate.",
    "examples": ["Es256", "Es384", "Es512", "Ps256", "Ed25519"]
},
```

2. Change the `manifest` property from `type: object` to `type: string`:
```json
"manifest": {
    "type": "string",
    "contentMediaType": "application/json",
    "description": "The manifest definition in crJSON format, as a raw JSON string. In YAML files this is written as a literal block scalar (|). The value is passed directly to the c2pa-rs manifest builder without further parsing."
},
```

3. Remove the nested `properties`, `additionalProperties: true`, `items`, etc. that were inside the old `manifest` object definition — the new `manifest` is a flat string.

4. Update the `examples` array at the bottom: change the manifest value from a JSON object to a JSON string, and add `"alg": "Es256"` at the top level.

- [ ] **Step 2: Validate the schema compiles**

```bash
cargo build 2>&1 | tail -5
```

(Schema is not compiled by Rust, but ensure no build breakage.)

- [ ] **Step 3: Commit**

```bash
git add schemas/test-case.schema.json
git commit -m "docs: update test-case JSON Schema for YAML format

- Add top-level alg property (optional, documents valid algorithm values)
- Change manifest from type:object to type:string with contentMediaType:application/json"
```

---

## Task 9: Rewrite `schemas/test-case-description.md`

**Files:**
- Modify: `schemas/test-case-description.md`

- [ ] **Step 1: Rewrite the document**

Replace the entire content of `schemas/test-case-description.md` with the following:

````markdown
# C2PA Validator Test Case Description

## Overview

This document describes the YAML-based format for defining **C2PA test cases** used in automated test asset generation. Each test case fully specifies how to produce a C2PA-signed test asset from an input media file.

The format is formally defined in [`schemas/test-case.schema.json`](test-case.schema.json) using [JSON Schema (Draft 2020-12)](https://json-schema.org/draft/2020-12).

### Purpose

Test cases enable automated, repeatable creation of C2PA-signed test assets. A test harness uses a test case to:

1. Take the specified **input asset** and embed a **manifest** (in crJSON format).
2. Sign the manifest with the specified **signing certificate** (and optionally obtain a timestamp from a **TSA**).
3. Produce a signed output asset ready for use in conformance testing.

---

## Format

Test case files are YAML. The outer envelope uses YAML syntax. The `manifest` field is a raw JSON string written as a YAML literal block scalar (`|`) and passed directly to the c2pa-rs manifest builder.

---

## Schema Reference

Only the properties listed below are permitted at the top level (`additionalProperties: false`).

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `testId` | string | ✅ | Unique identifier. SHOULD use dot-notation (e.g., `validator.claimSignature.valid`). Only alphanumeric characters, hyphens, underscores, and dots are allowed. |
| `title` | string | | Short, human-readable title. |
| `description` | string | | Detailed explanation of what the test case verifies. |
| `alg` | string | | Signing algorithm: `Es256`, `Es384`, `Es512`, `Ps256`, or `Ed25519`. If omitted, the tool auto-detects the algorithm from `signingCert`. |
| `inputAsset` | string | | Relative path to the input media asset. Optional — can be omitted when the input file is supplied on the command line. The command-line value takes precedence when both are present. An error is returned if neither is provided at runtime. |
| `signingCert` | string | ✅ | Relative path to the PEM-encoded X.509 signing certificate (and chain) used to sign the manifest. |
| `signingKey` | string | | Relative path to the PEM-encoded private key for `signingCert`. If omitted, the implementation obtains the key by another means (e.g., from the same file as `signingCert`). |
| `tsaUrl` | string (URI) | | URL of an RFC 3161-compliant Time-Stamping Authority. If omitted, no timestamp token is embedded. |
| `manifest` | string | ✅ | The manifest definition in crJSON format, written as a YAML literal block scalar (`|`). The value is passed directly to the c2pa-rs manifest builder. |

All paths in `inputAsset`, `signingCert`, and `signingKey` are resolved relative to the test case file's directory.

---

## Examples

### Example 1 — Valid, Trusted Asset

```yaml
testId: validator.claimSignature.valid
title: Valid Claim Signature
description: Produces an asset with a valid claim signature signed by a trusted certificate.
alg: Es256
signingCert: certs/test-signing.pem
signingKey: certs/test-signing.key
tsaUrl: http://timestamp.digicert.com
manifest: |
  {
    "claim_generator_info": [
      { "name": "crTool", "version": "0.1.0", "specVersion": "2.3" }
    ],
    "title": "Sample Test Asset",
    "assertions": [
      {
        "label": "c2pa.actions",
        "data": {
          "actions": [
            {
              "action": "c2pa.created",
              "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
            }
          ]
        }
      }
    ]
  }
```

### Example 2 — Asset with Ingredient

```yaml
testId: validator.ingredient.parentOf
title: Asset with Parent Ingredient
description: Produces an asset whose manifest references a parent ingredient.
signingCert: certs/test-signing.pem
signingKey: certs/test-signing.key
manifest: |
  {
    "claim_generator_info": [
      { "name": "crTool", "version": "0.1.0", "specVersion": "2.3" }
    ],
    "assertions": [
      {
        "label": "c2pa.actions",
        "data": { "actions": [{ "action": "c2pa.edited" }] }
      }
    ],
    "ingredients": [
      {
        "file_path": "assets/original.jpg",
        "relationship": "parentOf"
      }
    ]
  }
```

### Example 3 — Development Certificate (No TSA, No alg)

```yaml
testId: validator.audio.aiGenerated.devCert
title: AI-Generated Audio — Development Certificate
description: Produces an AI-generated audio asset. Algorithm auto-detected from certificate.
signingCert: certs/dev-signing.pem
signingKey: certs/dev-signing.key
manifest: |
  {
    "claim_generator_info": [
      { "name": "crTool", "version": "0.1.0", "specVersion": "2.3" }
    ],
    "format": "audio/mpeg",
    "assertions": [
      {
        "label": "c2pa.actions",
        "data": {
          "actions": [
            {
              "action": "c2pa.created",
              "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"
            }
          ]
        }
      }
    ]
  }
```

---

## File Layout Convention

```
test-suite/
├── assets/                  # Raw input media assets
│   └── sample.jpg
├── certs/                   # Signing certificates and private keys
│   ├── test-signing.pem
│   └── test-signing.key
└── test-cases/              # Test case description files (one per test)
    ├── validator.claimSignature.valid.yaml
    └── validator.ingredient.parentOf.yaml
```

---

## Validation

Test case description files SHOULD be validated against the JSON Schema before use:

```
schemas/test-case.schema.json
```
````

- [ ] **Step 2: Commit**

```bash
git add schemas/test-case-description.md
git commit -m "docs: rewrite test-case-description.md for YAML format

- Document YAML envelope + JSON block scalar manifest
- Add alg as a top-level field with valid values
- Update all three examples to YAML syntax"
```

---

## Task 10: Update `README.md` and `tests/README.md`

**Files:**
- Modify: `README.md`
- Modify: `tests/README.md`

- [ ] **Step 1: Update `README.md`**

Make these targeted changes:

1. In the first bullet under the intro, change:
   ```
   **Create test assets** — sign media files with C2PA manifests from test-case JSON files
   ```
   to:
   ```
   **Create test assets** — sign media files with C2PA manifests from test-case YAML files
   ```

2. In the `### Create test assets` section, update the examples:
   ```bash
   # Single test case
   c2pa-test-tool --create-test test-cases/positive/tc-created.yaml --output output/tc-created.jpg

   # Glob pattern — multiple test cases, output to directory
   c2pa-test-tool --create-test "test-cases/positive/tc-*.yaml" --output output/

   # Override the input asset on the command line
   c2pa-test-tool --create-test tc.yaml tests/fixtures/assets/raw/Dog.jpg --output output/
   ```

3. Rename the `## Test-case JSON format` section to `## Test-case YAML format`.

4. Replace the JSON example block with a YAML example:

   ```yaml
   testId: tc-created
   title: Created image
   alg: Es256
   signingCert: ../tests/fixtures/certs/es256_cert.pem
   signingKey: ../tests/fixtures/certs/es256_private.pem
   manifest: |
     {
       "claim_generator_info": [{ "name": "crTool", "version": "0.1.0" }],
       "assertions": [
         { "label": "c2pa.actions", "data": { "actions": [{ "action": "c2pa.created" }] } }
       ]
     }
   ```

5. Update the options table row for `--create-test`:
   ```
   | `-t / --create-test PATTERN` | Path or glob to test-case YAML file(s) |
   ```

- [ ] **Step 2: Update `tests/README.md`**

Add the `test_creation` suite to the **Integration Tests** section. After the existing `test_dog_webp_full_manifest` bullet, add:

```markdown
### Test Case Creation Tests (`tests/src/test_creation.rs`)

- ✅ `test_all_testfiles_creation` — Runs every YAML file in `testfiles-creation/` through the CLI, rotating across five input formats (JPEG, PNG, WebP, SVG, MP4). Asserts each run succeeds and produces a non-empty output file.
```

- [ ] **Step 3: Commit**

```bash
git add README.md tests/README.md
git commit -m "docs: update README and tests/README for YAML test case format"
```

---

## Task 11: Final Verification

- [ ] **Step 1: Run the full test suite one last time**

```bash
cargo test -- --test-threads=1 2>&1
```

Expected: all tests pass across all three test binaries (`integration_tests`, `test_profile_evaluation`, `test_creation`).

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -20
```

Expected: no warnings.

- [ ] **Step 3: Check formatting**

```bash
cargo fmt -- --check 2>&1
```

Expected: no output (already formatted).

- [ ] **Step 4: Verify no JSON files remain in testfiles-creation/**

```bash
ls testfiles-creation/*.json 2>&1
```

Expected: `No such file or directory`

- [ ] **Step 5: Verify YAML file count matches expectations**

```bash
ls testfiles-creation/*.yaml | wc -l
```

Expected: `41`

---

## Task 12: Create PR

- [ ] **Step 1: Push the branch**

```bash
git push -u origin feature/yaml-test-cases
```

- [ ] **Step 2: Create the PR**

```bash
gh pr create \
  --title "Migrate test case format from JSON to YAML" \
  --body "$(cat <<'EOF'
## Summary

- Converts all 41 test case files in `testfiles-creation/` from JSON to YAML
- The `manifest` field stays as a raw JSON literal block scalar (`|`), passed directly to c2pa-rs
- Adds top-level `alg` field to the YAML envelope (was never in manifest envelope; previously always auto-detected)
- Updates `TestCase` struct: `manifest: String`, new `alg: Option<String>`, parser uses `serde_yaml`
- Adds `test_creation` integration test suite: runs every YAML file through the CLI, rotating across 5 input formats (JPEG, PNG, WebP, SVG, MP4)
- Updates schema, test-case-description, README, and tests/README

## Motivation

YAML gives the outer envelope a cleaner human-readable form. The `manifest` stays as JSON because c2pa-rs consumes it as crJSON directly. Keeping `manifest` as an opaque string also removes any accidental round-trip conversion that could silently alter field ordering or numeric precision.

## Breaking Change

Test case files must now use `.yaml` extension and YAML envelope syntax. Any tooling that consumed the old `.json` files must be updated.

## Test Plan

- [ ] `cargo test -- --test-threads=1` passes all tests
- [ ] `test_all_testfiles_creation` exercises all 41 test cases across 5 media formats
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt -- --check` clean
- [ ] No `.json` files remain in `testfiles-creation/`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Record the PR URL**

```bash
gh pr view --web
```
