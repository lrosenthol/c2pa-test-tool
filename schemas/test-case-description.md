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
