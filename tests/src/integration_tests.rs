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
use std::fs;
use std::io::Write;

mod common;

use common::{output_dir, sign_file_with_manifest_and_ingredients};

// ============================================================================
// Output file sanity check
// ============================================================================

#[test]
fn test_output_files_are_readable() {
    let output = output_dir();
    if output.exists() {
        for entry in std::fs::read_dir(output).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let metadata = std::fs::metadata(&path).unwrap();
                assert!(
                    metadata.len() > 0,
                    "Output file should not be empty: {:?}",
                    path
                );
            }
        }
    }
}

// ============================================================================
// Ingredient error handling
// ============================================================================

#[test]
fn test_ingredient_missing_file_error() -> Result<()> {
    let manifest_content = r#"{
        "claim_generator_info": [{"name": "test", "version": "1.0.0"}],
        "title": "Test",
        "ingredients": [
            {
                "title": "Missing",
                "relationship": "parentOf",
                "file_path": "../tests/fixtures/assets/nonexistent.jpg"
            }
        ]
    }"#;

    let temp_manifest = output_dir().join("temp_manifest_missing.json");
    let mut file = fs::File::create(&temp_manifest)?;
    file.write_all(manifest_content.as_bytes())?;

    let input = common::testfiles_dir().join("Dog.jpg");
    let output = output_dir().join("should_fail.jpg");
    let ingredients_base_dir = temp_manifest.parent().unwrap();

    let result = sign_file_with_manifest_and_ingredients(
        &input,
        &output,
        &temp_manifest,
        ingredients_base_dir,
    );

    assert!(result.is_err(), "Should fail with missing ingredient file");

    if temp_manifest.exists() {
        fs::remove_file(&temp_manifest)?;
    }

    println!("✓ Missing ingredient file error handling test passed");
    Ok(())
}
