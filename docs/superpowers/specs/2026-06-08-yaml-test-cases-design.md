# Design: Migrate Test Case Format from JSON to YAML

**Date:** 2026-06-08
**Status:** Approved

## Problem

Test case files in `testfiles-creation/` are JSON. JSON requires quoting every key, forbids trailing commas and comments, and makes the manifest block — which must stay as raw crJSON — awkward to distinguish from the outer envelope. YAML gives the envelope a cleaner human-readable form while a literal block scalar keeps the manifest as unambiguous JSON.

## Approach

Clean break: delete all `.json` test cases, convert to `.yaml`, update the parser to use `serde_yaml`. No dual-format support.

## Data Format

The outer envelope is YAML. The `manifest` field is a raw JSON string using a YAML literal block scalar (`|`), passed directly to the c2pa-rs API unchanged.

A new top-level `alg` field in the YAML envelope specifies the signing algorithm. None of the existing test case files carry `alg` at the manifest envelope level (the two occurrences of `"alg"` in the JSON files are inside assertion data payloads, not signing configuration). Adding `alg` at the envelope level eliminates any need to parse the manifest JSON at runtime to determine the signing algorithm.

### Example

```yaml
testId: c2pa.test.actions-created
alg: Es256
signingCert: ../tests/fixtures/certs/es256_cert.pem
signingKey: ../tests/fixtures/certs/es256_private.pem
tsaUrl: http://timestamp.digicert.com
manifest: |
  {
    "claim_generator_info": [
      { "name": "crTool", "version": "0.1.0", "specVersion": "2.3" }
    ],
    "title": "p-actions-created",
    "assertions": [
      {
        "label": "c2pa.actions",
        "data": {
          "actions": [
            {
              "action": "c2pa.created",
              "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia",
              "when": "2026-01-17T14:44:19.231Z"
            }
          ]
        },
        "created": true
      }
    ],
    "ingredients": []
  }
```

## Code Changes

### `Cargo.toml`

Move `serde_yaml = "0.9"` from `[dev-dependencies]` to `[dependencies]`. The `TestCase` struct and `handle_create_test` live in `src/`, compiled into the binary, so `serde_yaml` must be a regular dependency. Add a new `[[test]]` entry for `test_creation` with `path = "tests/src/test_creation.rs"`.

### `src/test_case.rs`

- Change `manifest` field type: `serde_json::Value` → `String`
- Add top-level `alg: Option<String>` field to `TestCase`
- Replace `serde_json::from_str` with `serde_yaml::from_str` for parsing the test case file
- Remove `serde_json::to_string(&test_case.manifest)` — use `test_case.manifest.trim()` where a `&str` is needed, or `test_case.manifest.trim().to_owned()` where a `String` is needed (YAML `|` block scalars include a trailing `\n`; trimming ensures the string is valid JSON before passing to c2pa-rs)
- Update `alg` detection: use `test_case.alg` directly; fall back to auto-detecting from certificate when absent. Valid `alg` values are the same as before: `Es256`, `Es384`, `Es512`, `Ps256`, `Ed25519` (matched by the existing `parse_signing_algorithm` function)
- Update error messages to read "YAML file" instead of "JSON file"

### `src/main.rs`

Update the `--create-test` argument help text: "JSON file(s)" → "YAML file(s)".

### `schemas/test-case.schema.json`

- Add top-level `alg` property (type: string, optional; valid values: `Es256`, `Es384`, `Es512`, `Ps256`, `Ed25519`)
- Change `manifest` type from `object` to `string`; add `"contentMediaType": "application/json"` annotation and update the description to note it is a raw crJSON string passed directly to c2pa-rs

## Test Case File Conversion

Convert all 41 `.json` files in `testfiles-creation/` to `.yaml`:

- Rename each file: `<name>.json` → `<name>.yaml`
- Convert the outer envelope fields to YAML syntax
- Wrap the `manifest` value in a `|` literal block scalar preserving the JSON exactly
- Add `alg:` as a new top-level YAML field (the existing manifest JSON does not contain a signing `alg` field; this is new)

## New Integration Tests (`tests/src/test_creation.rs`)

Discover every `*.yaml` file in `testfiles-creation/` and run each through `handle_create_test`. Rotate deterministically through five input assets to exercise multiple media formats:

```rust
const INPUT_ASSETS: &[&str] = &[
    "tests/fixtures/assets/raw/Dog.jpg",
    "tests/fixtures/assets/raw/Dog.png",
    "tests/fixtures/assets/raw/Dog.webp",
    "tests/fixtures/assets/raw/Ghostscript_tiger.svg",
    "tests/fixtures/assets/raw/video1_with_thumb.mp4",
];
```

Each test asserts:
1. `handle_create_test` returns `Ok`
2. The output file exists
3. The output file size is greater than zero

Use `index % INPUT_ASSETS.len()` for asset selection — stable across runs, distributes all formats across the 41 test cases. All five formats are listed as supported in the README (`svg`, `mp4`, `jpg`, `png`, `webp`) and are supported by c2pa-rs with the `file_io` feature.

The new test file imports shared utilities from `tests/src/common/mod.rs` (specifically `output_dir()` and `fixtures_dir()`). Declare `mod common;` at the top of `test_creation.rs` as the other test files do.

## Documentation Changes

| File | Change |
|------|--------|
| `README.md` | Update `--create-test` examples to `*.yaml`; rename "Test-case JSON format" section to "Test-case YAML format" with a YAML example |
| `schemas/test-case-description.md` | Full rewrite: describe YAML envelope + JSON block scalar manifest; move `alg` to top-level field table; update all examples |
| `schemas/test-case.schema.json` | As described above |
| `tests/README.md` | Add `test_creation` suite description |
| `src/main.rs` | Update CLI help string for `--create-test` |

## Branch & PR

Work on a new branch `feature/yaml-test-cases`. PR description covers: motivation, format change summary, struct changes, new integration test suite, and documentation updates.
