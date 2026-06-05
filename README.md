# c2pa-test-tool

A CLI tool for working with [C2PA (Coalition for Content Provenance and Authenticity)](https://c2pa.org) Content Credentials. Provides three core capabilities:

1. **Create test assets** — sign media files with C2PA manifests from test-case JSON files
2. **Validate assets** — validate assets against a YAML grammar *(scaffolded; grammar TBD)*
3. **Profile evaluation** — evaluate crJSON indicators against a YAML asset profile

## Prerequisites

This project depends on `c2pa-rs` cloned as a sibling directory:

```
parent/
  c2pa-test-tool/        ← this repo
  c2pa-rs/               ← https://github.com/contentauth/c2pa-rs
  profile-evaluator-rs/  ← profile evaluator library
```

## Build

```bash
cargo build
cargo build --release
```

## Usage

```
c2pa-test-tool [OPTIONS] [INPUT_FILE]...
```

### Create test assets

Sign a media file with a C2PA manifest defined in a test-case JSON file:

```bash
# Single test case
c2pa-test-tool --create-test test-cases/positive/tc-created.json --output output/tc-created.jpg

# Glob pattern — multiple test cases, output to directory
c2pa-test-tool --create-test "test-cases/positive/tc-*.json" --output output/

# Override the input asset on the command line
c2pa-test-tool --create-test tc.json tests/fixtures/assets/raw/Dog.jpg --output output/
```

### Validate assets *(scaffold)*

```bash
c2pa-test-tool --validate image.jpg
c2pa-test-tool --validate --grammar grammar.yml image.jpg image2.png
```

### Profile evaluation

Evaluate crJSON indicators against a YAML asset profile:

```bash
c2pa-test-tool --profile profiles/real-life-capture_profile.yml indicators.json
c2pa-test-tool --profile profiles/real-life-capture_profile.yml --report-format yaml indicators.json
```

### Batch mode

Run multiple commands from a JSON batch file:

```bash
c2pa-test-tool --batch batch.json
```

### Trust list validation

Add `--trust` to any operation to fetch and apply the official C2PA and Content Credentials trust lists:

```bash
c2pa-test-tool --trust --profile profiles/... indicators.json
```

### Options

| Flag | Description |
|------|-------------|
| `-t / --create-test PATTERN` | Path or glob to test-case JSON file(s) |
| `--validate` | Validate input assets *(scaffold; grammar TBD)* |
| `--grammar FILE` | YAML grammar for validation (use with `--validate`) |
| `--profile FILE` | YAML asset profile for profile evaluation |
| `--report-format json\|yaml` | Profile report format (default: `json`) |
| `-o / --output PATH` | Output file or directory |
| `--trust` | Enable C2PA trust list validation |
| `-b / --batch FILE` | Run commands from a batch JSON file |
| `-q / --quiet` | Suppress progress output |
| `-l / --log FILE` | Write progress to a log file |

## Test-case JSON format

Test-case files follow the schema in `INTERNAL/schemas/test-case.schema.json`:

```json
{
  "testId": "tc-created",
  "title": "Created image",
  "inputAsset": "../tests/fixtures/assets/raw/Dog.jpg",
  "manifest": { ... },
  "signingCert": "../tests/fixtures/certs/es256_cert.pem",
  "signingKey":  "../tests/fixtures/certs/es256_private.pem"
}
```

All paths in a test-case file are resolved relative to the test-case file's directory.

## Tests

```bash
cargo test
cargo test -- --test-threads=1   # recommended for integration tests
```

## Supported asset formats

`avi avif c2pa dng gif heic heif jpg jpeg m4a mov mp3 mp4 pdf png svg tif tiff wav webp`

## License

Apache 2.0 — see [LICENSE](LICENSE).
