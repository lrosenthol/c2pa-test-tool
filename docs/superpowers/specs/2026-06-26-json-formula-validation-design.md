# Design: Replace Manifest Predicate DSL with JSON Formula Expressions

**Date:** 2026-06-26
**Status:** Approved

## Overview

Replace the custom predicate DSL in each `manifests:` entry of validation test-case YAMLs with a single [JSON Formula](https://opensource.adobe.com/json-formula/) string expression. The formula evaluates against the manifest's raw `validationResults` object from crJSON and must return a truthy value for the manifest check to pass.

## Motivation

The current `manifests:` DSL (`isEmpty`, `containsAllOf`, `containsNoneOf`, etc.) is a bespoke predicate language with limited expressiveness. JSON Formula is a spec-backed, string-based expression language (combining JMESPath queries with spreadsheet-style functions) that:

- Is already present in the dependency tree (via `profile-evaluator-rs` → `json-formula-rs`)
- Handles arbitrary boolean logic, not just the fixed set of predicates
- Is human-readable: `length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0`
- Eliminates a custom DSL and its evaluation code from this codebase

## Schema Change

### Before (per manifest entry)

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

### After (per manifest entry)

```yaml
manifests:
  - formula: "length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0"
```

Each `manifests:` array entry contains exactly one `formula` field (a JSON Formula expression string). The array ordering is unchanged: active manifest first, matching crJSON manifest order.

## Formula Evaluation Context

Each formula is evaluated against the manifest's raw `validationResults` object from crJSON:

```json
{
  "success": [{"code": "claimSignature.validated", "url": "..."}, ...],
  "failure": [{"code": "claimSignature.mismatch", "url": "..."}, ...],
  "informational": [{"code": "...", "url": "..."}, ...]
}
```

The formula must return a truthy JSON value (`true`, a non-zero number, a non-empty string) to pass. A falsy result (`false`, `null`, `0`, `""`) or an evaluation error is a failure.

## Example Translations

The preferred syntax for checking whether a code is present in an array is the JMESPath filter projection:
`length(success[?code == 'claimSignature.validated']) > 0`

This is unambiguous across JSON Formula implementations: filter the `success` array to entries where `.code` equals the target string, then check the count is greater than zero.

| Test case | Formula |
|---|---|
| `png_valid` | `length(failure) = 0 && length(success[?code == 'claimSignature.validated']) > 0 && length(success[?code == 'signingCredential.trusted']) > 0` |
| `wrong_signing_key` | `length(failure[?code == 'claimSignature.mismatch']) > 0 && length(success[?code == 'claimSignature.validated']) = 0` |
| `ingredient_with_hard_binding_mismatch` manifest[0] | `length(failure) = 0 && length(success[?code == 'signingCredential.trusted']) > 0 && length(success[?code == 'claimSignature.insideValidity']) > 0 && length(success[?code == 'claimSignature.validated']) > 0` |
| `ingredient_with_hard_binding_mismatch` manifest[1] | `length(failure[?code == 'assertion.dataHash.mismatch']) > 0 && length(success[?code == 'assertion.dataHash.match']) = 0` |

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
3. **Add** `json-formula-rs` as a direct dependency in `Cargo.toml`.
4. **Replace** per-manifest predicate evaluation in `run_validation()` with:
   - Parse the formula string via `json_formula_rs` (or equivalent API)
   - Evaluate against the manifest's `validationResults` JSON value
   - Treat truthy result as pass, falsy/error as fail
   - On error: record the error message as the failure reason

### `tests/validation/validation_test.schema.json`

Replace the `ManifestResult` JSON schema definition (note: this is the schema object named `ManifestResult`, not the Rust struct `ManifestResult` in `src/validation.rs` which is the report output type and is unchanged):

```json
"ManifestResult": {
  "description": "Expectation for a single manifest, expressed as a JSON Formula string.",
  "type": "object",
  "properties": {
    "formula": {
      "type": "string",
      "description": "A JSON Formula expression evaluated against the manifest's validationResults object. Must return truthy to pass."
    }
  },
  "required": ["formula"],
  "additionalProperties": false
}
```

Remove `StatusCodesExpectations`, `StatusCodeSet` definitions.

### Test YAML files

All existing `tests/validation/*.yaml` files are rewritten to use `formula:` entries (see example translations above). The `inputs:` section is unchanged.

## Error Reporting

When a formula fails, the report includes:
- The formula string that was evaluated
- Whether it returned falsy or threw an error (with error message)
- The actual `validationResults` values (successes, failures, informationals) for diagnosis

## Edge Cases

### Empty `manifests: []`

An empty `manifests:` list (zero entries) retains its existing meaning: the test expects the asset to contain no C2PA manifests. This behavior is unchanged — no formula is evaluated; the test passes if and only if crJSON contains zero manifests.

### Formula error reporting format

When a manifest formula fails, the `reasons` field of the `ManifestResult` report struct contains entries in the following formats:
- Falsy result: `"formula returned falsy: <formula-string>"`
- Evaluation error: `"formula error: <error-message> (formula: <formula-string>)"`

The actual `validationResults` successes, failures, and informationals are also included in the report output for diagnosis (existing behavior preserved).

## Non-Goals

- No migration path or backward compatibility with the old predicate DSL — it is removed entirely.
- No change to the `inputs:` section of the YAML.
- No change to how manifests are ordered or indexed.
