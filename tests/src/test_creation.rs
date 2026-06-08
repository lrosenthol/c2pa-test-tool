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

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::{cli_binary_path, output_dir};

/// Input assets rotated across test cases to exercise multiple media formats.
/// Selection is index-based (deterministic) so runs are reproducible.
const INPUT_ASSETS: &[(&str, &str)] = &[
    ("tests/fixtures/assets/raw/Dog.jpg", "jpg"),
    ("tests/fixtures/assets/raw/Dog.png", "png"),
    ("tests/fixtures/assets/raw/Dog.webp", "webp"),
    ("tests/fixtures/assets/raw/Ghostscript_tiger.svg", "svg"),
    ("tests/fixtures/assets/raw/video1_with_thumb.mp4", "mp4"),
];

/// Run every YAML test case in testfiles-creation/ through the CLI and assert
/// that a non-empty output asset is produced.
#[test]
fn test_all_testfiles_creation() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let testfiles_dir = manifest_dir.join("testfiles-creation");
    let out_dir = output_dir();
    let binary = cli_binary_path();

    let mut yaml_files: Vec<PathBuf> = std::fs::read_dir(&testfiles_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    yaml_files.sort();

    assert!(
        !yaml_files.is_empty(),
        "No YAML test case files found in testfiles-creation/ — was the conversion run?"
    );

    let mut failures = Vec::new();

    for (i, yaml_path) in yaml_files.iter().enumerate() {
        let (asset_rel, ext) = INPUT_ASSETS[i % INPUT_ASSETS.len()];
        let input_path = manifest_dir.join(asset_rel);
        let stem = yaml_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_path = out_dir.join(format!("tc-{}-{}.{}", i, stem, ext));

        let result = Command::new(&binary)
            .args([
                "--create-test",
                yaml_path.to_str().unwrap(),
                input_path.to_str().unwrap(),
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()?;

        if !result.status.success() {
            failures.push(format!(
                "  FAIL {:?}\n    stdout: {}\n    stderr: {}",
                yaml_path.file_name().unwrap(),
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            ));
            continue;
        }

        if !output_path.exists() {
            failures.push(format!(
                "  FAIL {:?} — output file not created",
                yaml_path.file_name().unwrap()
            ));
            continue;
        }

        if output_path.metadata()?.len() == 0 {
            failures.push(format!(
                "  FAIL {:?} — output file is empty",
                yaml_path.file_name().unwrap()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} test case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );

    println!(
        "✓ All {} test case files processed successfully",
        yaml_files.len()
    );
    Ok(())
}
