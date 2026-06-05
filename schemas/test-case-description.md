# C2PA Validator Test Case Description

## Overview

This document describes the JSON-based grammar for defining **C2PA test cases** used in automated test asset generation. Each test case fully specifies how to produce a C2PA-signed test asset from an input media file.

The grammar is formally defined in [`INTERNAL/schemas/test-case.schema.json`](test-case.schema.json) using [JSON Schema (Draft 2020-12)](https://json-schema.org/draft/2020-12).

### Purpose

Test cases enable automated, repeatable creation of C2PA-signed test assets. A test harness can use a test case to:

1. Take the specified **input asset** and embed a **manifest** (in crJSON format).
2. Sign the manifest with the specified **signing certificate** (and optionally obtain a timestamp from a **TSA**).
3. Produce a signed output asset ready for use in conformance testing.

---

## Schema Reference

The schema file is located at:

```
INTERNAL/schemas/test-case.schema.json
```

Only the properties listed below are permitted at the top level (`additionalProperties: false`).

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `testId` | string | ✅ | Unique identifier for the test case. SHOULD use dot-notation (e.g., `validator.claimSignature.valid`). Only alphanumeric characters, hyphens, underscores, and dots are allowed. |
| `title` | string | | Short, human-readable title. |
| `description` | string | | Detailed explanation of what the test case verifies. |
| `inputAsset` | string | | Relative path to the raw media asset (image, video, audio, document) into which the manifest will be embedded. Optional — can be omitted when the input file is supplied on the command line. When both are present, the command-line value takes precedence. An error is returned if neither is provided at runtime. |
| `manifest` | object | ✅ | Manifest declaration/definition in crJSON format. Specifies what content to embed during test asset generation. See [Manifest Object](#manifest-object). |
| `signingCert` | string | ✅ | Relative path to the PEM-encoded X.509 signing certificate (and chain) used to sign the manifest. |
| `signingKey` | string | | Relative path to the PEM-encoded private key corresponding to `signingCert`. If omitted, the implementation must obtain the private key by another means (e.g., from a hardware security module or from the same file as `signingCert`). |
| `tsaUrl` | string (URI) | | URL of an RFC 3161-compliant Time-Stamping Authority. If omitted, no timestamp token is embedded in the signed asset. |

---

### Manifest Object

The `manifest` field contains a crJSON manifest definition — the same format accepted by tools such as [c2patool](https://github.com/contentauth/c2patool). Additional crJSON manifest properties not listed below are permitted (`additionalProperties: true`).

| Field | Type | Description |
|-------|------|-------------|
| `alg` | string | Signing algorithm (e.g., `"Es256"`, `"Es384"`, `"Es512"`, `"Ps256"`, `"Ed25519"`). |
| `claim_generator_info` | array | Structured metadata about the claim generator software. See [Claim Generator Info](#claim-generator-info). |
| `assertions` | array | Array of C2PA assertion objects. See [Assertions](#assertions). |
| `ingredients` | array | Array of ingredient objects for source assets. See [Ingredients](#ingredients). |

#### Claim Generator Info

Each entry in `claim_generator_info` MUST have a `name` field and MAY include additional properties.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✅ | The name of the claim generator software. |
| `version` | string | | The version of the claim generator software. |
| `specVersion` | string | | The C2PA specification version implemented by the software (e.g., `"2.3"`). |

#### Assertions

Each entry in `assertions` MUST have both `label` and `data`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `label` | string | ✅ | The assertion type label (e.g., `"c2pa.actions"`, `"c2pa.hash.data"`). |
| `data` | any | ✅ | The assertion data payload. Structure depends on the assertion type. |

#### Ingredients

Each entry in `ingredients` describes a source asset used in producing the output asset.

| Field | Type | Description |
|-------|------|-------------|
| `file_path` | string | Path to an external asset file to load as a C2PA ingredient. When present, the file is read from disk and embedded; the entry is not passed as an inline ingredient definition. |
| `label` | string | A unique identifier for this ingredient. SHOULD use dot-notation (e.g., `"c2pa.ingredient.source"`). |
| `relationship` | string | Relationship between this ingredient and the output asset. MUST be `"parentOf"` or `"componentOf"`. |

Additional ingredient properties are permitted (`additionalProperties: true`).

---

## Examples

### Example 1 — Valid, Trusted Asset

Creates a test asset with a valid signature and trusted certificate.

```json
{
  "testId": "validator.claimSignature.valid",
  "title": "Valid Claim Signature",
  "description": "Produces an asset with a valid claim signature signed by a trusted certificate for use in validator conformance testing.",
  "inputAsset": "assets/sample.jpg",
  "manifest": {
    "alg": "Es256",
    "claim_generator_info": [
      {
        "name": "crTool",
        "version": "0.1.0",
        "specVersion": "2.3"
      }
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
  },
  "signingCert": "certs/test-signing.pem",
  "signingKey": "certs/test-signing.key",
  "tsaUrl": "http://timestamp.digicert.com"
}
```

### Example 2 — Asset with Ingredient

Creates a test asset that references a parent ingredient.

```json
{
  "testId": "validator.ingredient.parentOf",
  "title": "Asset with Parent Ingredient",
  "description": "Produces an asset whose manifest references a parent ingredient.",
  "inputAsset": "assets/derived.jpg",
  "manifest": {
    "alg": "Es256",
    "claim_generator_info": [
      {
        "name": "crTool",
        "version": "0.1.0",
        "specVersion": "2.3"
      }
    ],
    "assertions": [
      {
        "label": "c2pa.actions",
        "data": {
          "actions": [
            {
              "action": "c2pa.edited"
            }
          ]
        }
      }
    ],
    "ingredients": [
      {
        "file_path": "assets/original.jpg",
        "relationship": "parentOf"
      }
    ]
  },
  "signingCert": "certs/test-signing.pem",
  "signingKey": "certs/test-signing.key"
}
```

### Example 3 — Development Certificate (No TSA)

Creates a test asset signed with a development certificate and no timestamp.

```json
{
  "testId": "validator.audio.aiGenerated.devCert",
  "title": "AI-Generated Audio — Development Certificate",
  "description": "Produces an AI-generated audio asset signed with a development certificate and no timestamp token.",
  "inputAsset": "assets/ai-generated-audio.mp3",
  "manifest": {
    "alg": "Es256",
    "claim_generator_info": [
      {
        "name": "crTool",
        "version": "0.1.0",
        "specVersion": "2.3"
      }
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
  },
  "signingCert": "certs/dev-signing.pem",
  "signingKey": "certs/dev-signing.key"
}
```

---

## File Layout Convention

A test suite is recommended to follow this directory structure:

```
test-suite/
├── assets/                  # Raw input media assets
│   ├── sample.jpg
│   └── ai-generated-audio.mp3
├── certs/                   # Signing certificates and private keys
│   ├── test-signing.pem
│   ├── test-signing.key
│   ├── dev-signing.pem
│   └── dev-signing.key
└── test-cases/              # Test case description files (one per test)
    ├── validator.claimSignature.valid.json
    ├── validator.ingredient.parentOf.json
    └── validator.audio.aiGenerated.devCert.json
```

All paths in `inputAsset`, `signingCert`, and `signingKey` are resolved relative to the test case JSON file's directory. `inputAsset` may be omitted from the JSON and supplied instead as a positional argument on the command line, which allows the same test case file to be reused across different input assets.

---

## Validation

Test case description files SHOULD be validated against the JSON Schema before use:

```
INTERNAL/schemas/test-case.schema.json
```

The CI workflow in this repository automatically lints all `*.schema.json` files on commit using the [sourcemeta/jsonschema](https://github.com/sourcemeta/jsonschema) linter.
