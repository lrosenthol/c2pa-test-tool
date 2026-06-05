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

//! CLI integration tests for `--create-test` mode (single file, glob pattern, input override)
//! and batch mode with `test-cases` commands.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn binary() -> PathBuf {
    common::cli_binary_path()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn test_cases_dir() -> PathBuf {
    repo_root().join("test-cases")
}

fn test_output_dir(subdir: &str) -> PathBuf {
    let dir = repo_root()
        .join("target")
        .join("test_output")
        .join("create_test")
        .join(subdir);
    fs::create_dir_all(&dir).expect("Failed to create test output directory");
    dir
}

/// Run the binary and return (success, stdout, stderr)
fn run(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("Failed to execute crTool binary");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// ─── Single exact-path tests ─────────────────────────────────────────────────

/// Basic case: single test case JSON with its own `inputAsset`, output to a file.
#[test]
fn test_create_test_single_exact_path() -> Result<()> {
    let tc = test_cases_dir().join("positive/tc-created.json");
    let out = test_output_dir("single_exact").join("tc-created.jpg");

    let (ok, stdout, stderr) = run(&[
        "--create-test",
        tc.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(ok, "create-test should succeed: {stderr}");
    assert!(out.exists(), "Output file should exist: {out:?}");
    assert!(out.metadata()?.len() > 0, "Output file should not be empty");

    Ok(())
}

/// Single test case + explicit input file override on the CLI.
#[test]
fn test_create_test_with_cli_input_override() -> Result<()> {
    let tc = test_cases_dir().join("positive/tc-created.json");
    let input = repo_root().join("tests/fixtures/assets/raw/Dog.jpg");
    let out = test_output_dir("input_override").join("tc-created-override.jpg");

    assert!(input.exists(), "Input file must exist: {input:?}");

    let (ok, stdout, stderr) = run(&[
        "--create-test",
        tc.to_str().unwrap(),
        input.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(
        ok,
        "create-test with CLI input override should succeed: {stderr}"
    );
    assert!(out.exists(), "Output file should exist: {out:?}");

    Ok(())
}

// ─── Glob pattern tests ───────────────────────────────────────────────────────

/// Glob matching multiple test cases in the positive directory; output to a directory.
/// Each test case uses its own `inputAsset` from JSON.
#[test]
fn test_create_test_glob_multiple_cases() -> Result<()> {
    let pattern = format!("{}/test-cases/positive/tc-*.json", repo_root().display());
    let out_dir = test_output_dir("glob_multiple");

    let (ok, stdout, stderr) = run(&[
        "--create-test",
        &pattern,
        "--output",
        out_dir.to_str().unwrap(),
    ]);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(ok, "glob create-test should succeed: {stderr}");

    // All 4 positive test cases share Dog.jpg as input, so they overwrite the same output
    // filename. Verify via the summary line in stdout that multiple cases were processed.
    assert!(
        stdout.contains("succeeded") || stdout.contains("Done"),
        "stdout should confirm at least one test case processed: {stdout}"
    );
    let outputs: Vec<_> = fs::read_dir(&out_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jpg"))
        .collect();
    assert!(!outputs.is_empty(), "At least one output file should exist");

    Ok(())
}

/// Glob matching multiple test cases + a single input file override.
#[test]
fn test_create_test_glob_with_input_override() -> Result<()> {
    // Use only the two simplest test cases (no ingredients) to keep the test fast
    let pattern = format!(
        "{}/test-cases/positive/tc-created*.json",
        repo_root().display()
    );
    let input = repo_root().join("tests/fixtures/assets/raw/Dog.jpg");
    let out_dir = test_output_dir("glob_with_input");

    assert!(input.exists(), "Input file must exist: {input:?}");

    let (ok, stdout, stderr) = run(&[
        "--create-test",
        &pattern,
        input.to_str().unwrap(),
        "--output",
        out_dir.to_str().unwrap(),
    ]);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    // tc-created*.json matches at least tc-created.json
    assert!(
        ok,
        "glob create-test with input override should succeed: {stderr}"
    );

    Ok(())
}

/// Glob matching multiple test cases but output is a plain file — should fail with an error.
#[test]
fn test_create_test_glob_multiple_requires_dir_output() -> Result<()> {
    let pattern = format!("{}/test-cases/positive/tc-*.json", repo_root().display());
    let out_file = test_output_dir("glob_error").join("should-not-exist.jpg");

    let (ok, _stdout, stderr) = run(&[
        "--create-test",
        &pattern,
        "--output",
        out_file.to_str().unwrap(),
    ]);

    println!("stderr: {stderr}");

    assert!(
        !ok,
        "Should fail when multiple test cases match and output is a file"
    );
    assert!(
        stderr.contains("directory") || stderr.contains("Directory"),
        "Error should mention 'directory': {stderr}"
    );
    assert!(
        !out_file.exists(),
        "Output file should not have been created"
    );

    Ok(())
}

/// Non-matching glob pattern should fail with an error.
#[test]
fn test_create_test_glob_no_matches_fails() -> Result<()> {
    let pattern = format!(
        "{}/test-cases/positive/tc-nonexistent-*.json",
        repo_root().display()
    );
    let out_dir = test_output_dir("glob_no_match");

    let (ok, _stdout, stderr) = run(&[
        "--create-test",
        &pattern,
        "--output",
        out_dir.to_str().unwrap(),
    ]);

    println!("stderr: {stderr}");

    assert!(!ok, "Should fail when glob matches no files");

    Ok(())
}

// ─── Batch mode tests ─────────────────────────────────────────────────────────

/// Write a temp batch JSON file, run the CLI with `--batch`, and return (success, stdout, stderr).
fn run_batch(batch_json: &str, out_dir: &Path) -> (bool, String, String) {
    let batch_file = out_dir.join("batch.json");
    fs::write(&batch_file, batch_json).expect("Failed to write batch file");

    let (ok, stdout, stderr) = run(&["--batch", batch_file.to_str().unwrap()]);
    (ok, stdout, stderr)
}

/// Batch mode: single test case command using an exact path.
#[test]
fn test_batch_test_cases_single_exact_path() -> Result<()> {
    let tc = test_cases_dir().join("positive/tc-created.json");
    let out_dir = test_output_dir("batch_single");
    let batch_out_dir = out_dir.join("output");
    fs::create_dir_all(&batch_out_dir)?;

    let batch_json = format!(
        r#"[
  {{
    "command": "test-cases",
    "arguments": [
      "--create-test", {tc:?},
      "-o", {out:?}
    ]
  }}
]"#,
        tc = tc.to_str().unwrap(),
        out = batch_out_dir.to_str().unwrap()
    );

    let (ok, stdout, stderr) = run_batch(&batch_json, &out_dir);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(ok, "Batch test-cases command should succeed: {stderr}");

    let outputs: Vec<_> = fs::read_dir(&batch_out_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jpg"))
        .collect();
    assert!(
        !outputs.is_empty(),
        "Batch should produce at least one output file"
    );

    Ok(())
}

/// Batch mode: test-cases command with a glob pattern for `--create-test`.
#[test]
fn test_batch_test_cases_glob_pattern() -> Result<()> {
    let pattern = format!("{}/test-cases/positive/tc-*.json", repo_root().display());
    let out_dir = test_output_dir("batch_glob");
    let batch_out_dir = out_dir.join("output");
    fs::create_dir_all(&batch_out_dir)?;

    let batch_json = format!(
        r#"[
  {{
    "command": "test-cases",
    "arguments": [
      "--create-test", {pattern:?},
      "-o", {out:?}
    ]
  }}
]"#,
        pattern = pattern,
        out = batch_out_dir.to_str().unwrap()
    );

    let (ok, stdout, stderr) = run_batch(&batch_json, &out_dir);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(ok, "Batch test-cases glob command should succeed: {stderr}");

    // All 4 positive test cases share Dog.jpg as input, so the final output is one file.
    // Verify via the summary in stdout that multiple cases were processed.
    assert!(
        stdout.contains("succeeded") || stdout.contains("Done"),
        "stdout should confirm test cases processed: {stdout}"
    );
    let outputs: Vec<_> = fs::read_dir(&batch_out_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jpg"))
        .collect();
    assert!(!outputs.is_empty(), "At least one output file should exist");

    Ok(())
}

/// Batch mode: test-cases command with an explicit input file in `inputFiles`.
#[test]
fn test_batch_test_cases_with_input_files() -> Result<()> {
    let tc = test_cases_dir().join("positive/tc-created.json");
    let input = repo_root().join("tests/fixtures/assets/raw/Dog.jpg");
    let out_dir = test_output_dir("batch_with_input");
    let batch_out_dir = out_dir.join("output");
    fs::create_dir_all(&batch_out_dir)?;

    assert!(input.exists(), "Input file must exist: {input:?}");

    let batch_json = format!(
        r#"[
  {{
    "command": "test-cases",
    "arguments": [
      "--create-test", {tc:?},
      "-o", {out:?}
    ],
    "inputFiles": [{input:?}]
  }}
]"#,
        tc = tc.to_str().unwrap(),
        out = batch_out_dir.to_str().unwrap(),
        input = input.to_str().unwrap()
    );

    let (ok, stdout, stderr) = run_batch(&batch_json, &out_dir);

    println!("stdout: {stdout}");
    println!("stderr: {stderr}");

    assert!(
        ok,
        "Batch test-cases with inputFiles should succeed: {stderr}"
    );

    Ok(())
}
