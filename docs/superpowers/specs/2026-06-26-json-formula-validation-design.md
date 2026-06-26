# Design: Replace Manifest Predicate DSL with JSON Formula Expressions

**Date:** 2026-06-26
**Status:** Draft (revised)

## Overview

Replace the custom predicate DSL in each `manifests:` entry of validation test-case YAMLs with a single [JSON Formula](https://opensource.adobe.com/json-formula/) string expression. Additionally, add top-level `globals:` and `expressions:` fields (consistent with the rubric/profile YAML format) and a top-level `formula:` field for the zero-manifest case.

## Motivation

The current `manifests:` DSL (`isEmpty`, `containsAllOf`, `containsNoneOf`, etc.) is a bespoke predicate language with limited expressiveness. JSON Formula is a spec-backed, string-based expression language (combining JMESPath queries with spreadsheet-style functions) that:

- Is already present in the dependency tree (via `profile-evaluator-rs` → `json-formula-rs`)
- Handles arbitrary boolean logic, not just the fixed set of predicates
- Is human-readable: `length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0`
- Eliminates a custom DSL and its evaluation code from this codebase
- Aligns the validation YAML format with the existing rubric/profile YAML conventions

## Schema Change

### Top-level structure

```yaml
description: "Human-readable description of the test case"

inputs:                          # unchanged
  assetPath: ./asset.png
  claimSignerTrustListPaths: [certs/root_ca1_cert.pem]
  tsaTrustListPaths: [certs/root_ca1_cert.pem]
  validationTime: "2001-06-01T00:00:00Z"

globals:                         # optional; key→value map passed as globals to all formulas
  $my_codes:
    - claimSignature.validated
    - signingCredential.trusted

expressions:                     # optional; named sub-expressions available in all formulas
  _vr: "(@.validationResults)"

# Exactly one of `formula:` or `manifests:` must be present:

formula: "..."                   # for zero-manifest case; evaluated against full crJSON

manifests:                       # for per-manifest checks; one entry per manifest
  - formula: "..."
```

`formula:` (top-level) and `manifests:` are mutually exclusive. If `manifests:` is present it must be non-empty.

### Per-manifest entry (before → after)

**Before:**
```yaml
manifests:
  - failures:
      isEmpty: {}
    successes:
      containsAllOf:
        codes:
          - claimSignature.validated
          - signingCredential.trusted
```

**After:**
```yaml
manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0"
```

Each `manifests:` array entry contains exactly one `formula` field. The array ordering is unchanged: active manifest first, matching crJSON manifest order.

## Formula Evaluation Contexts

### Per-manifest formula (inside `manifests:`)

Evaluated against the manifest's raw `validationResults` object from crJSON:

```json
{
  "success": [{"code": "claimSignature.validated", "url": "..."}, ...],
  "failure": [{"code": "claimSignature.mismatch", "url": "..."}, ...],
  "informational": [{"code": "...", "url": "..."}, ...]
}
```

### Top-level `formula:`

Evaluated against the full crJSON document:

```json
{
  "manifests": [...],
  "active_manifest": "...",
  ...
}
```

Used when the test expects no C2PA manifests in the asset, e.g.:
```yaml
formula: "length(manifests) = 0"
```

### Globals and expressions

`globals:` values are accessible in any formula via `$name`. `expressions:` entries are named sub-expressions, also accessible as globals (consistent with how `profile-evaluator-rs` handles rubric expressions). Both are passed to `jf.search()` as the globals argument, pre-processed by the evaluator before formula execution.

### Truthiness

All example formulas return a boolean via comparison operators (`>`, `=`, `==`). If a formula returns a non-boolean, JSON Formula's native truthiness applies: non-zero numbers, non-empty strings, non-empty arrays, and non-empty objects are truthy. A falsy result (`false`, `null`, `0`, `""`, `[]`) or an evaluation error is a failure.

## Example Translations

The preferred syntax for checking whether a status code is present:
`length(success[?code == 'claimSignature.validated']) > 0`

| Test case | Formula (in `manifests:` entry or top-level `formula:`) |
|---|---|
| `png_valid` | `length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0` |
| `wrong_signing_key` | `length(failure[?code == 'claimSignature.mismatch']) > 0 && length(success[?code == 'claimSignature.validated']) = 0` |
| `ingredient_with_hard_binding_mismatch` manifest[0] | `length(failure) = 0 && length(success[?code == 'signingCredential.trusted']) > 0 && length(success[?code == 'claimSignature.insideValidity']) > 0 && length(success[?code == 'claimSignature.validated']) > 0` |
| `ingredient_with_hard_binding_mismatch` manifest[1] | `length(failure[?code == 'assertion.dataHash.mismatch']) > 0 && length(success[?code == 'assertion.dataHash.match']) = 0` |
| asset with no manifests | top-level `formula: "length(manifests) = 0"` |

## Rust Implementation Changes

### `src/validation.rs`

1. **Remove** `StatusCodesExpectations`, `StatusCodeSet`, `EmptyObject` structs and their `check()` impl.
2. **Replace** `ManifestExpectation` with:
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct ManifestExpectation {
       pub formula: String,
   }
   ```
3. **Add** `globals` and `expressions` fields to `ValidationTestCase`:
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct ValidationTestCase {
       pub description: String,
       pub inputs: Inputs,
       #[serde(default)]
       pub globals: serde_json::Value,           // object or null
       #[serde(default)]
       pub expressions: IndexMap<String, String>, // name → expression string
       pub formula: Option<String>,              // top-level; mutually exclusive with manifests
       pub manifests: Option<Vec<ManifestExpectation>>,
       #[serde(rename = "validatorSpecVersions", default)]
       pub validator_spec_versions: Vec<String>,
   }
   ```
   Validation at parse time: error if both `formula` and `manifests` are present, or if neither is present.
4. **Add** `json-formula-rs = "=0.2.0"` as a direct dependency in `Cargo.toml`.
5. **Build globals** before evaluation: merge `expressions` entries into the globals object so named sub-expressions are accessible from all formulas (consistent with how `profile-evaluator-rs` processes rubric expressions).
6. **Evaluate per-manifest formulas** using:
   ```rust
   use json_formula_rs::{JsonFormula, JsonFormulaError};

   let jf = JsonFormula::new();
   // validation_results_json = manifest_json["validationResults"].clone()
   // merged_globals = globals merged with expressions entries
   let result = jf.search(&expectation.formula, &validation_results_json,
                           Some(&merged_globals), None);
   ```
7. **Evaluate top-level formula** (when `formula:` is present) with the full crJSON as data:
   ```rust
   let result = jf.search(&formula, &crjson_value, Some(&merged_globals), None);
   ```
8. Treat truthy result as pass, falsy/error as fail. On error, record the error message as a failure reason.

### `tests/validation/validation_test.schema.json`

**Replace** the `ManifestResult` JSON schema definition (this is the schema object, not the Rust report struct `ManifestResult` in `src/validation.rs` which is unchanged):

```json
"ManifestExpectation": {
  "description": "Formula expectation for a single manifest.",
  "type": "object",
  "properties": {
    "formula": {
      "type": "string",
      "description": "JSON Formula expression evaluated against the manifest's validationResults object. Must return truthy to pass."
    }
  },
  "required": ["formula"],
  "additionalProperties": false
}
```

**Add** top-level fields to the root schema object:

```json
"formula": {
  "type": "string",
  "description": "Top-level JSON Formula expression for the zero-manifest case, evaluated against the full crJSON. Mutually exclusive with manifests."
},
"globals": {
  "type": "object",
  "description": "Named values passed as globals to all formula evaluations. Keys accessible via $name in formulas."
},
"expressions": {
  "type": "object",
  "additionalProperties": { "type": "string" },
  "description": "Named sub-expressions available in all formulas. Merged into globals before evaluation."
}
```

**Remove** `StatusCodesExpectations`, `StatusCodeSet`, `ManifestResult` (old) definitions.

Update the root `required` to no longer require `manifests`.

`"additionalProperties": false` is intentional — any future fields require an explicit schema update.

### Test YAML files

All existing `tests/validation/*.yaml` files are rewritten to use `formula:` entries (see example translations above). The `inputs:` section is unchanged.

## Error Reporting

When a formula fails, the `reasons` field of the `ManifestResult` Rust report struct contains:
- Falsy result: `"formula returned falsy: <formula-string>"`
- Evaluation error: `"formula error: <error-message> (formula: <formula-string>)"`

The actual `validationResults` successes, failures, and informationals are still included in the report output for diagnosis (existing behavior preserved).

## Edge Cases

### Manifest count mismatch (per-manifest path)

When `manifests:` is present, if the YAML has more entries than crJSON has manifests, the test fails with: `"crJSON has N manifest(s) but test case expects at least M"`. If fewer entries than manifests, only the first N are checked (existing behavior).

### Top-level formula with non-empty manifests

It is a YAML authoring error to use `formula:` when the asset actually has manifests. The top-level formula evaluates against the full crJSON regardless — the author is responsible for the formula being meaningful.

## Non-Goals

- No migration path or backward compatibility with the old predicate DSL — it is removed entirely.
- No change to the `inputs:` section of the YAML.
- No change to manifest ordering or indexing.
