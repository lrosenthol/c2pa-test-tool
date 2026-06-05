# Content Credential Tool - Tests

## Overview

Integration tests for the Content Credential Tool. Tests verify that manifests can be embedded into various image formats (JPEG, PNG, WEBP) using different manifest configurations.

## ✅ Working with c2pa (Local Development)

**The tests now work perfectly with self-signed certificates!**

By using the local c2pa-rs repository (path dependency), we have access to the same test infrastructure that c2patool uses, including the Ed25519 test certificates that pass all validation.

## Running Tests

### Run All Tests
```bash
# Run tests sequentially (recommended to avoid race conditions)
cargo test -- --test-threads=1

# Run tests in parallel (faster, but may have occasional failures due to file access)
cargo test
```

### Run Specific Tests
```bash
# Test a specific image/manifest combination
cargo test test_dog_jpg_simple_manifest

# Test all PNG images
cargo test test_dog_png
```

## Test Structure

### Integration Tests (`tests/src/integration_tests.rs`)

- ✅ `test_dog_jpg_simple_manifest` - Embeds simple manifest into Dog.jpg
- ✅ `test_dog_jpg_full_manifest` - Embeds full manifest into Dog.jpg
- ✅ `test_dog_png_simple_manifest` - Embeds simple manifest into Dog.png
- ✅ `test_dog_png_full_manifest` - Embeds full manifest into Dog.png
- ✅ `test_dog_webp_simple_manifest` - Embeds simple manifest into Dog.webp
- ✅ `test_dog_webp_full_manifest` - Embeds full manifest into Dog.webp
- ✅ `test_all_images_both_manifests` - Batch test of all combinations

### Helper Tests (`tests/src/common/mod.rs::tests`)

- ✅ `test_fixtures_exist` - Verifies test directory structure
- ✅ `test_images_exist` - Verifies all test images are present
- ✅ `test_manifests_exist` - Verifies manifest JSON files exist
- ✅ `test_certs_exist` - Verifies test certificates exist

## How It Works

The tests use the exact same approach as c2pa-rs's own test suite:

1. **Ed25519 Test Certificates**: Uses the official Ed25519 test certificates from c2pa-rs (`tests/fixtures/certs/ed25519.*`)
2. **CallbackSigner**: Creates a `CallbackSigner` that signs data without requiring certificate chain validation
3. **Test Mode**: The c2pa library automatically disables strict certificate validation in test mode

## Test Certificates

The test certificates are copied from the c2pa-rs repository:
```
tests/fixtures/certs/
├── ed25519.pem     # Private key (from c2pa-rs)
└── ed25519.pub     # Certificate chain (from c2pa-rs)
```

These certificates are specifically designed for testing and pass all c2pa validation requirements.

## Test Output

Successful tests create signed files in:
```
target/test_output/
├── Dog_simple.jpg    (35 KB)
├── Dog_full.jpg      (35 KB)
├── Dog_simple.png    (1.2 MB)
├── Dog_full.png      (1.2 MB)
├── Dog_simple.webp   (2.9 MB)
└── Dog_full.webp     (2.9 MB)
```

## Verifying Signed Output

To verify a signed image:

```bash
# Install c2patool from the local repository
cd ../c2pa-rs/cli
cargo install --path .

# View manifest
c2patool ../../crTool/target/test_output/Dog_simple.jpg

# Get JSON output
c2patool ../../crTool/target/test_output/Dog_simple.jpg -o manifest.json
```

## Known Issues

### Race Condition in Parallel Tests

When tests run in parallel, there's a small chance of a "failed to fill whole buffer" error on PNG files. This is because multiple tests try to access the same input file simultaneously.

**Solution**: Run tests sequentially with `--test-threads=1`

```bash
cargo test -- --test-threads=1
```

## CI/CD Setup

For continuous integration:

```yaml
# .github/workflows/test.yml
- name: Run tests
  run: cargo test -- --test-threads=1
```

## Development Notes

### Using Local c2pa-rs

The project uses a path dependency to the local c2pa-rs repository:

```toml
[dependencies]
c2pa = { path = "../c2pa-rs/sdk", features = ["file_io"] }
```

This gives us:
- ✅ Latest c2pa features from your local checkout
- ✅ Access to test infrastructure
- ✅ Same certificates as c2patool
- ✅ No certificate validation issues

### Switching to Published Version

To use the published c2pa crate instead:

```toml
[dependencies]
c2pa = { version = "0.75", features = ["file_io"] }
```

Note: You'll need to provide your own test certificates if using the published version.

## References

- [C2PA Specification](https://c2pa.org/specifications/)
- [c2pa-rs Repository](https://github.com/contentauth/c2pa-rs)
- [c2pa-rs Documentation](https://docs.rs/c2pa/)
- [Ed25519 Signature Scheme](https://ed25519.cr.yp.to/)
