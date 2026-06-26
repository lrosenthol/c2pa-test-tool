# Google-contributed validator test cases

These are validator test cases produced using Google-internal tooling.

Please report test issues via
https://github.com/c2pa-org/public-testfiles/issues/new

## Contents:

  *  `assets/` - media files, in various formats
  *  `certs/` - PEM-format x.509 certificates
  *  `tests/` - validator test cases referencing the files in `assets` and
     `certificates`
  
## Test case schema

Test cases are YAML files following the JSON schema in `validation_test.schema.json`. Top-level fields:

* `description` — description of the test case
* `inputs` — validator inputs
  * `assetPath` — path to the asset to validate
  * `claimSignerTrustListPaths` — paths to PEM files for the claim signer trust list
  * `tsaTrustListPaths` — paths to PEM files for the TSA trust list
  * `validationTime` — validation time in RFC 3339 format (e.g. `"2010-03-21T15:30:00Z"`)
* `globals` *(optional)* — named values passed to all formula evaluations; accessible in formulas as `$name`
* `expressions` *(optional)* — named sub-expressions (formula strings) merged into globals and callable from any formula
* `validatorSpecVersions` *(optional)* — C2PA spec version(s) this test applies to (e.g. `"2.4"`); empty means all versions

Exactly one of the following must be present:

* `formula` — a single [JSON Formula](https://opensource.adobe.com/json-formula/) string evaluated against the **full crJSON document**. Used when the test expects no C2PA manifests, e.g. `length(manifests) = 0`.

* `manifests` — an array of per-manifest checks, active manifest first (matching crJSON order). Each entry has one field:
  * `formula` — a JSON Formula string evaluated against the manifest's raw `validationResults` object: `{success: [{code, url}], failure: [{code, url}], informational: [{code, url}]}`. Must return truthy to pass.

**Note:** json-formula-rs requires double-quoted strings in filter expressions (`"claimSignature.validated"`, not `'claimSignature.validated'` — single quotes silently return no results). Use YAML's `>-` block scalar to write multi-line formulas without quoting conflicts:

```yaml
manifests:
- formula: >-
    length(failure) = 0 &&
    length(success[?code == "claimSignature.validated"]) > 0
```

### Example formulas

Check that a manifest has no failures and specific success codes:
```
length(failure) = 0 && length(success[?code == "claimSignature.validated"]) > 0
```

Check that a specific failure code is present:
```
length(failure[?code == "claimSignature.mismatch"]) > 0
```

Check that the asset has no C2PA manifests (top-level `formula:`):
```
length(manifests) = 0
```

This is not an officially supported Google product. This project is not eligible
for the [Google Open Source Software Vulnerability Rewards Program](https://bughunters.google.com/open-source-security).
