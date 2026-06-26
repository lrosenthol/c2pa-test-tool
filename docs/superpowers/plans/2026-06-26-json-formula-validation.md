# JSON Formula Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the custom predicate DSL in validation test-case YAMLs with JSON Formula expressions, adding top-level `globals:`, `expressions:`, and `formula:` fields.

**Architecture:** `src/validation.rs` is the sole evaluation engine — remove the DSL structs (`StatusCodesExpectations`, `StatusCodeSet`, `EmptyObject`) and replace per-manifest evaluation with `json-formula-rs`. The `ValidationTestCase` struct gains `globals`, `expressions`, `formula` (top-level), and changes `manifests` to `Option<Vec<ManifestExpectation>>` where each entry holds a single `formula` string. All test YAMLs are rewritten in the new format; the test runner and report structs are unchanged.

**Tech Stack:** Rust, `json-formula-rs = "=0.2.0"` (already in Cargo.lock as transitive dep), `serde_json`, `serde_yaml`

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `json-formula-rs = "=0.2.0"` to `[dependencies]` |
| `src/validation.rs` | Major rewrite: remove DSL structs, update `ValidationTestCase`, rewrite `run_validation()` |
| `tests/src/test_validation.rs` | Remove unit tests for old DSL structs; update integration test for new report shape |
| `tests/validation/validation_test.schema.json` | Replace `ManifestResult`/`StatusCodesExpectations`/`StatusCodeSet`; add `formula`, `globals`, `expressions` to root |
| `tests/validation/png_valid.yaml` | Rewrite |
| `tests/validation/mp3_valid.yaml` | Rewrite |
| `tests/validation/mp4_valid.yaml` | Rewrite |
| `tests/validation/wrong_signing_key.yaml` | Rewrite |
| `tests/validation/unsupported_hashed_uri_algorithm.yaml` | Rewrite |
| `tests/validation/ingredient_with_hard_binding_mismatch.yaml` | Rewrite |

---

## Task 1: Add `json-formula-rs` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

  In `Cargo.toml`, add to `[dependencies]`:

  ```toml
  json-formula-rs = "=0.2.0"
  ```

- [ ] **Step 2: Verify it resolves**

  ```bash
  cargo check 2>&1 | head -20
  ```

  Expected: compiles cleanly (no new errors — `json-formula-rs` is already in `Cargo.lock`).

- [ ] **Step 3: Commit**

  ```bash
  git add Cargo.toml Cargo.lock
  git commit -m "chore: add json-formula-rs as direct dependency"
  ```

---

## Task 2: Rewrite `ValidationTestCase` struct

**Files:**
- Modify: `src/validation.rs`

This task updates the data model without touching evaluation logic. The old `ManifestExpectation` is replaced; `StatusCodesExpectations`, `StatusCodeSet`, `EmptyObject` stay for now (removed in Task 3).

- [ ] **Step 1: Write a failing compile test**

  In `tests/src/test_validation.rs`, add at the bottom:

  ```rust
  #[test]
  fn test_manifest_expectation_has_formula_field() {
      let exp = crtool::validation::ManifestExpectation {
          formula: "length(failure) = 0".to_string(),
      };
      assert_eq!(exp.formula, "length(failure) = 0");
  }

  #[test]
  fn test_validation_test_case_has_globals_and_expressions() {
      use indexmap::IndexMap;
      use serde_json::json;
      let tc = crtool::validation::ValidationTestCase {
          description: "test".to_string(),
          inputs: crtool::validation::Inputs {
              asset_path: "x.jpg".to_string(),
              claim_signer_trust_list_paths: vec![],
              tsa_trust_list_paths: vec![],
              validation_time: None,
          },
          globals: json!({}),
          expressions: IndexMap::new(),
          formula: Some("length(manifests) = 0".to_string()),
          manifests: None,
          validator_spec_versions: vec![],
      };
      assert!(tc.formula.is_some());
  }
  ```

- [ ] **Step 2: Run to confirm it fails**

  ```bash
  cargo test test_manifest_expectation_has_formula_field test_validation_test_case_has_globals_and_expressions -- --test-threads=1 2>&1 | tail -20
  ```

  Expected: compile error — `ManifestExpectation` has no `formula` field yet.

- [ ] **Step 3: Update `ValidationTestCase` and `ManifestExpectation` in `src/validation.rs`**

  Replace the existing `ManifestExpectation` struct (lines ~38–46):

  ```rust
  #[derive(Debug, Deserialize)]
  pub struct ManifestExpectation {
      pub formula: String,
  }
  ```

  Replace `ValidationTestCase` (lines ~17–24) — add `globals`, `expressions`, `formula`, make `manifests` optional. Use `IndexMap` (already in `Cargo.lock` as a transitive dep) so expression insertion order is preserved:

  First add to `Cargo.toml` `[dependencies]`:
  ```toml
  indexmap = "2"
  ```

  Then in `src/validation.rs`:

  ```rust
  use indexmap::IndexMap;

  #[derive(Debug, Deserialize)]
  pub struct ValidationTestCase {
      pub description: String,
      pub inputs: Inputs,
      #[serde(default)]
      pub globals: serde_json::Value,
      #[serde(default)]
      pub expressions: IndexMap<String, String>,
      pub formula: Option<String>,
      pub manifests: Option<Vec<ManifestExpectation>>,
      #[serde(rename = "validatorSpecVersions", default)]
      pub validator_spec_versions: Vec<String>,
  }
  ```

  Keep `StatusCodesExpectations`, `StatusCodeSet`, `EmptyObject` in place for now — they'll be removed in Task 3.

- [ ] **Step 4: Run the new tests**

  ```bash
  cargo test test_manifest_expectation_has_formula_field test_validation_test_case_has_globals_and_expressions -- --test-threads=1 2>&1 | tail -10
  ```

  Expected: both PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add src/validation.rs tests/src/test_validation.rs
  git commit -m "feat: update ValidationTestCase struct for json-formula fields"
  ```

---

## Task 3: Rewrite `run_validation()` with JSON Formula evaluation

**Files:**
- Modify: `src/validation.rs`

This task replaces the evaluation loop and removes the old DSL structs. After this task, `run_validation()` uses `json-formula-rs` for both per-manifest and top-level formula evaluation.

- [ ] **Step 1: Write failing unit tests for the new formula-based evaluation**

  In `tests/src/test_validation.rs`, replace the four old DSL unit tests (the `test_is_empty_*`, `test_contains_all_of_*` tests) with formula evaluation tests. Remove the old DSL imports at the top of the file too.

  Replace the old imports and tests:

  ```rust
  use json_formula_rs::JsonFormula;
  use serde_json::json;

  fn eval_formula(formula: &str, data: serde_json::Value) -> bool {
      eval_formula_with_globals(formula, data, None)
  }

  fn eval_formula_with_globals(
      formula: &str,
      data: serde_json::Value,
      globals: Option<serde_json::Value>,
  ) -> bool {
      let jf = JsonFormula::new();
      match jf.search(formula, &data, globals.as_ref(), None) {
          Ok(v) => crtool::validation::is_truthy(&v),
          Err(_) => false,
      }
  }

  #[test]
  fn test_formula_empty_failure_set_passes() {
      let data = json!({"success": [], "failure": [], "informational": []});
      assert!(eval_formula("length(failure) = 0", data));
  }

  #[test]
  fn test_formula_non_empty_failure_set_fails() {
      let data = json!({"success": [], "failure": [{"code": "claimSignature.mismatch", "url": ""}], "informational": []});
      assert!(!eval_formula("length(failure) = 0", data));
  }

  #[test]
  fn test_formula_code_present_in_success() {
      let data = json!({"success": [{"code": "claimSignature.validated", "url": ""}], "failure": [], "informational": []});
      assert!(eval_formula("length(success[?code == 'claimSignature.validated']) > 0", data));
  }

  #[test]
  fn test_formula_code_absent_from_success() {
      let data = json!({"success": [], "failure": [], "informational": []});
      assert!(!eval_formula("length(success[?code == 'claimSignature.validated']) > 0", data));
  }

  #[test]
  fn test_formula_globals_accessible_in_formula() {
      // Verify that a $global value injected via build_globals is reachable from a formula.
      let data = json!({"success": [{"code": "claimSignature.validated", "url": ""}], "failure": [], "informational": []});
      let globals = json!({"$target_code": "claimSignature.validated"});
      // $target_code is a string global; use it in a direct equality check
      assert!(eval_formula_with_globals(
          "length(success[?code == $target_code]) > 0",
          data,
          Some(globals),
      ));
  }
  ```

- [ ] **Step 2: Run to confirm tests fail**

  ```bash
  cargo test test_formula_ -- --test-threads=1 2>&1 | tail -20
  ```

  Expected: compile error — `is_truthy` not public yet.

- [ ] **Step 3: Rewrite `src/validation.rs` evaluation logic**

  At the top of `src/validation.rs`, add imports:

  ```rust
  use indexmap::IndexMap;
  use json_formula_rs::{JsonFormula, JsonFormulaError};
  ```

  Add a public `is_truthy` helper after the struct definitions:

  ```rust
  pub fn is_truthy(v: &serde_json::Value) -> bool {
      match v {
          serde_json::Value::Bool(b) => *b,
          serde_json::Value::Null => false,
          serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
          serde_json::Value::String(s) => !s.is_empty(),
          serde_json::Value::Array(a) => !a.is_empty(),
          serde_json::Value::Object(o) => !o.is_empty(),
      }
  }
  ```

  Build the globals merger helper (put it after `is_truthy`):

  ```rust
  fn build_globals(
      globals: &serde_json::Value,
      expressions: &IndexMap<String, String>,
  ) -> serde_json::Value {
      let mut merged = match globals.as_object() {
          Some(m) => m.clone(),
          None => serde_json::Map::new(),
      };
      for (name, expr) in expressions {
          merged.insert(name.clone(), serde_json::Value::String(expr.clone()));
      }
      serde_json::Value::Object(merged)
  }
  ```

  Replace the per-manifest evaluation block inside `run_validation()`. Find the section that starts with `for (i, expected) in test_case.manifests.iter().enumerate()` and replace the entire block from there to the end of the function with:

  ```rust
  let jf = JsonFormula::new();
  let merged_globals = build_globals(&test_case.globals, &test_case.expressions);
  let globals_arg = if merged_globals.as_object().map(|o| o.is_empty()).unwrap_or(true) {
      None
  } else {
      Some(merged_globals.clone())
  };

  match (&test_case.formula, &test_case.manifests) {
      (Some(_), Some(_)) => {
          anyhow::bail!("test case '{}': 'formula' and 'manifests' are mutually exclusive", test_case.description);
      }
      (None, None) => {
          anyhow::bail!("test case '{}': one of 'formula' or 'manifests' must be present", test_case.description);
      }
      (Some(top_formula), None) => {
          // Top-level formula: evaluate against full crJSON
          let result = jf.search(top_formula, &crjson_value, globals_arg.as_ref(), None);
          let (pass, reason) = formula_result_to_pass(result, top_formula);
          let overall_pass = pass;
          return Ok(ValidationReport {
              description: test_case.description,
              overall_pass,
              manifests: if pass {
                  vec![]
              } else {
                  vec![ManifestResult {
                      index: 0,
                      pass: false,
                      reasons: vec![reason],
                      actual_successes: vec![],
                      actual_failures: vec![],
                      actual_informationals: vec![],
                  }]
              },
              validation_time_ignored,
          });
      }
      (None, Some(expected_manifests)) => {
          let mut manifest_results = Vec::new();
          let mut overall_pass = true;

          for (i, expected) in expected_manifests.iter().enumerate() {
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

              let result = jf.search(&expected.formula, vr, globals_arg.as_ref(), None);
              let (pass, reason) = formula_result_to_pass(result, &expected.formula);

              if !pass {
                  overall_pass = false;
              }
              manifest_results.push(ManifestResult {
                  index: i,
                  pass,
                  reasons: if pass { vec![] } else { vec![reason] },
                  actual_successes,
                  actual_failures,
                  actual_informationals,
              });
          }

          return Ok(ValidationReport {
              description: test_case.description,
              overall_pass,
              manifests: manifest_results,
              validation_time_ignored,
          });
      }
  }
  ```

  Add the `formula_result_to_pass` helper near the bottom of the file (before `extract_codes`):

  ```rust
  fn formula_result_to_pass(
      result: Result<serde_json::Value, JsonFormulaError>,
      formula: &str,
  ) -> (bool, String) {
      match result {
          Ok(v) if is_truthy(&v) => (true, String::new()),
          Ok(_) => (false, format!("formula returned falsy: {}", formula)),
          Err(e) => (false, format!("formula error: {} (formula: {})", e, formula)),
      }
  }
  ```

  Also **remove** `StatusCodesExpectations`, `StatusCodeSet`, `EmptyObject` structs and their `impl` block. Remove the old `for (i, expected)` loop and the `if test_case.manifests.is_empty()` block that preceded it (the new `match` replaces both).

- [ ] **Step 4: Run the new unit tests**

  ```bash
  cargo test test_formula_ -- --test-threads=1 2>&1 | tail -15
  ```

  Expected: all four `test_formula_*` tests PASS.

- [ ] **Step 5: Run all tests to check for compile errors**

  ```bash
  cargo test -- --test-threads=1 2>&1 | tail -30
  ```

  The integration test (`test_run_all_validation_yaml_files`) will fail because the YAMLs still use the old format — that's expected. Confirm: no compile errors, only test runtime failures.

- [ ] **Step 6: Commit**

  ```bash
  git add src/validation.rs tests/src/test_validation.rs
  git commit -m "feat: replace predicate DSL with json-formula evaluation in run_validation"
  ```

---

## Task 4: Rewrite all test YAML files

**Files:**
- Modify: `tests/validation/png_valid.yaml`
- Modify: `tests/validation/mp3_valid.yaml`
- Modify: `tests/validation/mp4_valid.yaml`
- Modify: `tests/validation/wrong_signing_key.yaml`
- Modify: `tests/validation/unsupported_hashed_uri_algorithm.yaml`
- Modify: `tests/validation/ingredient_with_hard_binding_mismatch.yaml`

- [ ] **Step 1: Rewrite `png_valid.yaml`**

  ```yaml
  description: PNG with valid C2PA manifest
  inputs:
    assetPath: ./png_valid.png
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0"
  ```

- [ ] **Step 2: Rewrite `mp3_valid.yaml`**

  ```yaml
  description: MP3 with valid C2PA manifest
  inputs:
    assetPath: ./mp3_valid.mp3
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0"
  ```

- [ ] **Step 3: Rewrite `mp4_valid.yaml`**

  ```yaml
  description: MP4 with valid C2PA manifest
  inputs:
    assetPath: ./mp4_valid.mp4
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0"
  ```

- [ ] **Step 4: Rewrite `wrong_signing_key.yaml`**

  ```yaml
  description: claim signature created with a different key than the one in the cert
  inputs:
    assetPath: ./wrong_signing_key.jpg
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure[?code == 'claimSignature.mismatch']) > 0 && length(success[?code == 'claimSignature.validated']) = 0"
  ```

- [ ] **Step 5: Rewrite `unsupported_hashed_uri_algorithm.yaml`**

  ```yaml
  description: hashed URI algorithm unsupported (algorithm.unsupported)
  inputs:
    assetPath: ./unsupported_hashed_uri_algorithm.jpg
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure[?code == 'algorithm.unsupported']) > 0"
  ```

- [ ] **Step 6: Rewrite `ingredient_with_hard_binding_mismatch.yaml`**

  ```yaml
  description: JPEG with C2PA manifest, carrying a c2pa.opened action that references an ingredient with a data hash failure
  inputs:
    assetPath: ./ingredient_with_hard_binding_mismatch.jpg
    claimSignerTrustListPaths:
    - certs/root_ca1_cert.pem
    tsaTrustListPaths:
    - certs/root_ca1_cert.pem
    validationTime: "2001-06-01T00:00:00Z"
  manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'signingCredential.trusted']) > 0 && length(success[?code == 'claimSignature.insideValidity']) > 0 && length(success[?code == 'claimSignature.validated']) > 0"
  - formula: "length(failure[?code == 'assertion.dataHash.mismatch']) > 0 && length(success[?code == 'assertion.dataHash.match']) = 0"
  ```

- [ ] **Step 7: Run the integration test**

  ```bash
  cargo test test_run_all_validation_yaml_files -- --test-threads=1 2>&1 | tail -30
  ```

  Expected: the test itself PASSES. The test assets use a 2001 PKI; since `c2pa-rs` doesn't support `validationTime` override, certs appear expired. The test allows this by checking that any failures are *only* `signingCredential.expired` — if your output shows that code and nothing else unexpected, it's fine. The test fails if `run_validation` returns an `Err` (parse/IO error) or if a manifest fails with an unexpected code.

- [ ] **Step 8: Commit**

  ```bash
  git add tests/validation/
  git commit -m "feat: rewrite validation YAMLs to use json-formula expressions"
  ```

---

## Task 5: Update JSON schema

**Files:**
- Modify: `tests/validation/validation_test.schema.json`

- [ ] **Step 1: Rewrite the schema**

  Replace the entire contents of `tests/validation/validation_test.schema.json` with:

  ```json
  {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "title": "ValidationTestCase",
    "type": "object",
    "properties": {
      "description": {
        "type": "string"
      },
      "inputs": {
        "$ref": "#/definitions/Inputs"
      },
      "globals": {
        "type": "object",
        "description": "Named values passed as globals to all formula evaluations. Keys accessible as $name in formulas."
      },
      "expressions": {
        "type": "object",
        "additionalProperties": { "type": "string" },
        "description": "Named sub-expressions merged into globals and available in all formulas."
      },
      "formula": {
        "type": "string",
        "description": "Top-level JSON Formula expression evaluated against the full crJSON document. Used for the zero-manifest case. Mutually exclusive with manifests."
      },
      "manifests": {
        "description": "Per-manifest formula checks, in reverse order (active manifest first). Mutually exclusive with formula.",
        "type": "array",
        "items": {
          "$ref": "#/definitions/ManifestExpectation"
        }
      },
      "validatorSpecVersions": {
        "description": "C2PA spec version(s) that this test case applies to. If empty, applies to all versions.",
        "type": "array",
        "items": {
          "type": "string"
        }
      }
    },
    "additionalProperties": false,
    "required": [
      "description",
      "inputs"
    ],
    "definitions": {
      "Inputs": {
        "description": "Inputs for the validation.",
        "type": "object",
        "properties": {
          "assetPath": {
            "description": "Path to the asset file.",
            "type": "string"
          },
          "claimSignerTrustListPaths": {
            "description": "Paths to PEM files containing the claim signer trust list.",
            "type": "array",
            "items": { "type": "string" }
          },
          "tsaTrustListPaths": {
            "description": "Paths to PEM files containing the TSA trust list.",
            "type": "array",
            "items": { "type": "string" }
          },
          "validationTime": {
            "description": "Validation time in RFC 3339 format (e.g. \"2010-03-21T15:30:00Z\").",
            "type": "string"
          }
        },
        "additionalProperties": false,
        "required": ["assetPath", "claimSignerTrustListPaths", "tsaTrustListPaths", "validationTime"]
      },
      "ManifestExpectation": {
        "description": "Formula expectation for a single manifest.",
        "type": "object",
        "properties": {
          "formula": {
            "type": "string",
            "description": "JSON Formula expression evaluated against the manifest's raw validationResults object. Must return truthy to pass."
          }
        },
        "required": ["formula"],
        "additionalProperties": false
      }
    }
  }
  ```

- [ ] **Step 2: Run all tests one final time**

  ```bash
  cargo test -- --test-threads=1 2>&1 | tail -20
  ```

  Expected: all tests pass (or only fail with `signingCredential.expired` as handled by the existing test assertion).

- [ ] **Step 3: Run clippy and fmt**

  ```bash
  cargo fmt && cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -20
  ```

  Expected: no warnings or errors.

- [ ] **Step 4: Commit**

  ```bash
  git add tests/validation/validation_test.schema.json
  git commit -m "chore: update validation test schema for json-formula format"
  ```
