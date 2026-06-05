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
use std::path::{Path, PathBuf};

mod common;

use common::{
    get_test_images, has_asset_thumbnail, has_ingredient_thumbnails, manifests_dir, output_dir,
    sign_file_with_manifest, sign_file_with_manifest_and_ingredients,
    sign_file_with_manifest_and_options, verify_signed_file,
};

fn generate_output_name(input: &Path, manifest_type: &str, subdir: Option<&str>) -> PathBuf {
    let stem = input.file_stem().unwrap().to_str().unwrap();
    let ext = input.extension().unwrap().to_str().unwrap();
    let filename = format!("{}_{}.{}", stem, manifest_type, ext);

    if let Some(sub) = subdir {
        let dir = output_dir().join(sub);
        std::fs::create_dir_all(&dir).expect("Failed to create subdirectory");
        dir.join(filename)
    } else {
        output_dir().join(filename)
    }
}

// ============================================================================
// Basic signing tests — Dog.jpg / .png / .webp with various manifests
// ============================================================================

#[test]
fn test_dog_jpg_simple_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("simple_manifest.json");
    let output = generate_output_name(&input, "simple", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Created Image");
    }

    println!("✓ Dog.jpg with simple_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_jpg_full_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("full_manifest.json");
    let output = generate_output_name(&input, "full", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Edited Photo");
        assert!(!manifest.assertions().is_empty());
    }

    println!("✓ Dog.jpg with full_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_png_simple_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("simple_manifest.json");
    let output = generate_output_name(&input, "simple", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Created Image");
    }

    println!("✓ Dog.png with simple_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_png_full_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("full_manifest.json");
    let output = generate_output_name(&input, "full", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Edited Photo");
        assert!(!manifest.assertions().is_empty());
    }

    println!("✓ Dog.png with full_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_webp_simple_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.webp");
    let manifest = manifests_dir().join("simple_manifest.json");
    let output = generate_output_name(&input, "simple", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Created Image");
    }

    println!("✓ Dog.webp with simple_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_webp_full_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.webp");
    let manifest = manifests_dir().join("full_manifest.json");
    let output = generate_output_name(&input, "full", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Edited Photo");
        assert!(!manifest.assertions().is_empty());
    }

    println!("✓ Dog.webp with full_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_all_images_all_manifests() -> Result<()> {
    let manifests = vec![
        ("simple", manifests_dir().join("simple_manifest.json")),
        ("full", manifests_dir().join("full_manifest.json")),
        ("asset_ref", manifests_dir().join("asset_ref_manifest.json")),
        ("asset_type", manifests_dir().join("asset_type_manifest.json")),
        ("cloud_data", manifests_dir().join("cloud_data_manifest.json")),
        ("depthmap_gdepth", manifests_dir().join("depthmap_gdepth_manifest.json")),
        ("external_reference", manifests_dir().join("external_reference_manifest.json")),
        ("actions_v2_edited", manifests_dir().join("actions_v2_edited_manifest.json")),
        ("actions_v2_translated", manifests_dir().join("actions_v2_translated_manifest.json")),
        ("actions_v2_redacted", manifests_dir().join("actions_v2_redacted_manifest.json")),
        ("actions_v2_cropped", manifests_dir().join("actions_v2_cropped_manifest.json")),
        ("actions_v2_filtered", manifests_dir().join("actions_v2_filtered_manifest.json")),
    ];

    let mut success_count = 0;
    let mut total_count = 0;

    for input in get_test_images() {
        for (manifest_type, manifest_path) in &manifests {
            total_count += 1;
            let output = generate_output_name(&input, manifest_type, Some("batch"));

            match sign_file_with_manifest(&input, &output, manifest_path) {
                Ok(_) => match verify_signed_file(&output) {
                    Ok(_) => {
                        success_count += 1;
                        println!(
                            "✓ {} with {} manifest",
                            input.file_name().unwrap().to_str().unwrap(),
                            manifest_type
                        );
                    }
                    Err(e) => {
                        eprintln!("✗ Verification failed for {:?}: {}", output, e);
                    }
                },
                Err(e) => {
                    eprintln!(
                        "✗ Signing failed for {:?} with {}: {}",
                        input, manifest_type, e
                    );
                }
            }
        }
    }

    println!("\n{}/{} tests passed", success_count, total_count);
    assert_eq!(
        success_count, total_count,
        "All image/manifest combinations should succeed"
    );

    Ok(())
}

// ============================================================================
// Actions v2 manifest tests
// ============================================================================

#[test]
fn test_actions_v2_edited_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("actions_v2_edited_manifest.json");
    let output = generate_output_name(&input, "actions_v2_edited", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(
            manifest.title().unwrap_or_default(),
            "Image with Action V2 - Edited with Template"
        );
        let has_actions_v2 = manifest.assertions().iter().any(|a| a.label() == "c2pa.actions.v2");
        assert!(has_actions_v2, "Should have c2pa.actions.v2 assertion");
    }

    println!("✓ Dog.jpg with actions_v2_edited_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_actions_v2_translated_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("actions_v2_translated_manifest.json");
    let output = generate_output_name(&input, "actions_v2_translated", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(
            manifest.title().unwrap_or_default(),
            "Image with Action V2 - Translated"
        );
        let has_actions_v2 = manifest.assertions().iter().any(|a| a.label() == "c2pa.actions.v2");
        assert!(has_actions_v2, "Should have c2pa.actions.v2 assertion");
    }

    println!("✓ Dog.png with actions_v2_translated_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_actions_v2_redacted_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.webp");
    let manifest = manifests_dir().join("actions_v2_redacted_manifest.json");
    let output = generate_output_name(&input, "actions_v2_redacted", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(
            manifest.title().unwrap_or_default(),
            "Image with Action V2 - Redacted"
        );
        let has_actions_v2 = manifest.assertions().iter().any(|a| a.label() == "c2pa.actions.v2");
        assert!(has_actions_v2, "Should have c2pa.actions.v2 assertion");
    }

    println!("✓ Dog.webp with actions_v2_redacted_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_actions_v2_cropped_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("actions_v2_cropped_manifest.json");
    let output = generate_output_name(&input, "actions_v2_cropped", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(
            manifest.title().unwrap_or_default(),
            "Image with Action V2 - Cropped"
        );
        let has_actions_v2 = manifest.assertions().iter().any(|a| a.label() == "c2pa.actions.v2");
        assert!(has_actions_v2, "Should have c2pa.actions.v2 assertion");
    }

    println!("✓ Dog.jpg with actions_v2_cropped_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_actions_v2_filtered_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("actions_v2_filtered_manifest.json");
    let output = generate_output_name(&input, "actions_v2_filtered", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(
            manifest.title().unwrap_or_default(),
            "Image with Action V2 - Filtered with Multiple Actions"
        );
        let has_actions_v2 = manifest.assertions().iter().any(|a| a.label() == "c2pa.actions.v2");
        assert!(has_actions_v2, "Should have c2pa.actions.v2 assertion");
    }

    println!("✓ Dog.png with actions_v2_filtered_manifest.json: {}", output.display());
    Ok(())
}

// ============================================================================
// Special assertion manifests
// ============================================================================

#[test]
fn test_dog_jpg_external_reference_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("external_reference_manifest.json");
    let output = generate_output_name(&input, "external_reference", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Image with External Reference");
        let has = manifest.assertions().iter().any(|a| a.label() == "c2pa.external-reference");
        assert!(has, "Should have c2pa.external-reference assertion");
    }

    println!("✓ Dog.jpg with external_reference_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_jpg_cloud_data_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("cloud_data_manifest.json");
    let output = generate_output_name(&input, "cloud_data", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Image with Cloud-Hosted Assertion Data");
        let has = manifest.assertions().iter().any(|a| a.label() == "c2pa.cloud-data");
        assert!(has, "Should have c2pa.cloud-data assertion");
    }

    println!("✓ Dog.jpg with cloud_data_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_jpg_depthmap_gdepth_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("depthmap_gdepth_manifest.json");
    let output = generate_output_name(&input, "depthmap_gdepth", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Image with GDepth 3D Depth Map");
        let has = manifest.assertions().iter().any(|a| a.label() == "c2pa.depthmap.gdepth");
        assert!(has, "Should have c2pa.depthmap.gdepth assertion");
    }

    println!("✓ Dog.jpg with depthmap_gdepth_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_jpg_asset_type_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("asset_type_manifest.json");
    let output = generate_output_name(&input, "asset_type", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Image with Asset Type Information");
        let has = manifest.assertions().iter().any(|a| a.label() == "c2pa.asset-type");
        assert!(has, "Should have c2pa.asset-type assertion");
    }

    println!("✓ Dog.jpg with asset_type_manifest.json: {}", output.display());
    Ok(())
}

#[test]
fn test_dog_jpg_asset_ref_manifest() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("asset_ref_manifest.json");
    let output = generate_output_name(&input, "asset_ref", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;
    if let Some(manifest_label) = reader.active_label() {
        let manifest = reader.get_manifest(manifest_label).unwrap();
        assert_eq!(manifest.title().unwrap_or_default(), "Image with Asset Reference");
        let has = manifest.assertions().iter().any(|a| a.label() == "c2pa.asset-ref");
        assert!(has, "Should have c2pa.asset-ref assertion");
    }

    println!("✓ Dog.jpg with asset_ref_manifest.json: {}", output.display());
    Ok(())
}

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
// Ingredient tests
// ============================================================================

#[test]
fn test_simple_with_ingredient() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("simple_with_ingredient.json");
    let output = generate_output_name(&input, "simple_ingredient", Some("individual"));
    let ingredients_base_dir = manifest.parent().unwrap();

    sign_file_with_manifest_and_ingredients(&input, &output, &manifest, ingredients_base_dir)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest_store = reader.get_manifest(manifest_label).unwrap();
        let ingredients = manifest_store.ingredients();
        assert_eq!(ingredients.len(), 1, "Should have one ingredient");
        assert_eq!(ingredients[0].title().unwrap_or_default(), "Original Image");
    }

    println!("✓ Dog.png with simple_with_ingredient.json: {}", output.display());
    Ok(())
}

#[test]
fn test_with_ingredients_from_files() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.webp");
    let manifest = manifests_dir().join("with_ingredients_from_files.json");
    let output = generate_output_name(&input, "with_ingredients", Some("individual"));
    let ingredients_base_dir = manifest.parent().unwrap();

    sign_file_with_manifest_and_ingredients(&input, &output, &manifest, ingredients_base_dir)?;

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some());

    if let Some(manifest_label) = reader.active_label() {
        let manifest_store = reader.get_manifest(manifest_label).unwrap();
        let ingredients = manifest_store.ingredients();
        assert_eq!(ingredients.len(), 2, "Should have two ingredients");
        assert_eq!(ingredients[0].title().unwrap_or_default(), "Background Image");
        assert_eq!(*ingredients[0].relationship(), c2pa::Relationship::ParentOf);
        assert_eq!(ingredients[1].title().unwrap_or_default(), "Secondary Element");
        assert_eq!(*ingredients[1].relationship(), c2pa::Relationship::ComponentOf);
    }

    println!("✓ Dog.webp with with_ingredients_from_files.json: {}", output.display());
    Ok(())
}

#[test]
fn test_ingredient_missing_file_error() -> Result<()> {
    use std::io::Write;

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

// ============================================================================
// Thumbnail tests
// ============================================================================

#[test]
fn test_asset_thumbnail_not_present_by_default() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("simple_manifest.json");
    let output = generate_output_name(&input, "no_asset_thumb", Some("individual"));

    sign_file_with_manifest(&input, &output, &manifest)?;

    let reader = verify_signed_file(&output)?;

    if let Some(manifest_label) = reader.active_label() {
        assert!(
            !has_asset_thumbnail(&reader, manifest_label),
            "Asset thumbnail should NOT be present by default"
        );
    }

    println!("✓ Asset thumbnail correctly absent by default: {}", output.display());
    Ok(())
}

#[test]
fn test_asset_thumbnail_present_when_requested() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.jpg");
    let manifest = manifests_dir().join("simple_manifest.json");
    let output = generate_output_name(&input, "with_asset_thumb", Some("individual"));
    let ingredients_base_dir = manifest.parent().unwrap();

    sign_file_with_manifest_and_options(&input, &output, &manifest, ingredients_base_dir, true, false)?;

    let reader = verify_signed_file(&output)?;

    if let Some(manifest_label) = reader.active_label() {
        assert!(
            has_asset_thumbnail(&reader, manifest_label),
            "Asset thumbnail should be present when requested"
        );
    }

    println!("✓ Asset thumbnail correctly present when requested: {}", output.display());
    Ok(())
}

#[test]
fn test_ingredient_thumbnails_not_present_by_default() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("simple_with_ingredient.json");
    let output = generate_output_name(&input, "no_ing_thumb", Some("individual"));
    let ingredients_base_dir = manifest.parent().unwrap();

    sign_file_with_manifest_and_ingredients(&input, &output, &manifest, ingredients_base_dir)?;

    let reader = verify_signed_file(&output)?;

    if let Some(manifest_label) = reader.active_label() {
        assert!(
            !has_ingredient_thumbnails(&reader, manifest_label),
            "Ingredient thumbnails should NOT be present by default"
        );
    }

    println!("✓ Ingredient thumbnails correctly absent by default: {}", output.display());
    Ok(())
}

#[test]
fn test_ingredient_thumbnails_present_when_requested() -> Result<()> {
    let input = common::testfiles_dir().join("Dog.png");
    let manifest = manifests_dir().join("simple_with_ingredient.json");
    let output = generate_output_name(&input, "with_ing_thumb", Some("individual"));
    let ingredients_base_dir = manifest.parent().unwrap();

    sign_file_with_manifest_and_options(&input, &output, &manifest, ingredients_base_dir, false, true)?;

    let reader = verify_signed_file(&output)?;

    if let Some(manifest_label) = reader.active_label() {
        assert!(
            has_ingredient_thumbnails(&reader, manifest_label),
            "Ingredient thumbnails should be present when requested"
        );
    }

    println!("✓ Ingredient thumbnails correctly present when requested: {}", output.display());
    Ok(())
}

// ============================================================================
// Multi-file / CLI tests
// ============================================================================

#[test]
fn test_multiple_files_processing() -> Result<()> {
    use std::process::Command;

    let output_dir = common::output_dir().join("multi_file_test");
    fs::create_dir_all(&output_dir)?;

    let binary_path = common::cli_binary_path();
    let test_cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-cases");

    let tc1 = test_cases_dir.join("positive/tc-created.json");
    let tc2 = test_cases_dir.join("negative/tc-n-created-nodst.json");
    let output1 = output_dir.join("tc-created.jpg");
    let output2 = output_dir.join("tc-n-created-nodst.jpg");

    let result1 = Command::new(&binary_path)
        .arg("--create-test")
        .arg(&tc1)
        .arg("--output")
        .arg(&output1)
        .output()?;

    assert!(
        result1.status.success(),
        "First --create-test invocation failed: {}",
        String::from_utf8_lossy(&result1.stderr)
    );

    let result2 = Command::new(&binary_path)
        .arg("--create-test")
        .arg(&tc2)
        .arg("--output")
        .arg(&output2)
        .output()?;

    assert!(
        result2.status.success(),
        "Second --create-test invocation failed: {}",
        String::from_utf8_lossy(&result2.stderr)
    );

    let reader1 = verify_signed_file(&output1)?;
    assert!(reader1.active_label().is_some());

    let reader2 = verify_signed_file(&output2)?;
    assert!(reader2.active_label().is_some());

    println!("✓ Multiple --create-test invocations test passed");

    Ok(())
}

#[test]
fn test_create_test_with_ingredient() -> Result<()> {
    use std::process::Command;

    let output_dir = common::output_dir().join("create_test_ingredient");
    fs::create_dir_all(&output_dir)?;

    let binary_path = common::cli_binary_path();
    let test_cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-cases");

    let tc = test_cases_dir.join("positive/tc-placed-with-ingredient.json");
    let output = output_dir.join("tc-placed-with-ingredient.jpg");

    let result = Command::new(&binary_path)
        .arg("--create-test")
        .arg(&tc)
        .arg("--output")
        .arg(&output)
        .output()?;

    assert!(
        result.status.success(),
        "--create-test with ingredient failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let reader = verify_signed_file(&output)?;
    assert!(reader.active_label().is_some(), "Output file should have a C2PA manifest");

    println!("✓ --create-test with ingredient test passed");

    Ok(())
}

#[test]
fn test_multi_file_error_handling() -> Result<()> {
    use std::process::Command;

    let binary_path = common::cli_binary_path();
    let test_cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-cases");

    let nonexistent_tc = test_cases_dir.join("positive/nonexistent-test-case.json");
    let output = common::output_dir().join("multi_file_error_test/output.jpg");

    let result = Command::new(&binary_path)
        .arg("--create-test")
        .arg(&nonexistent_tc)
        .arg("--output")
        .arg(&output)
        .output()?;

    assert!(
        !result.status.success(),
        "Command should fail with non-existent test case file"
    );

    println!("✓ Multi-file error handling test passed");

    Ok(())
}

#[test]
fn test_create_test_missing_output_fails() -> Result<()> {
    use std::process::Command;

    let binary_path = common::cli_binary_path();
    let test_cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-cases");
    let tc = test_cases_dir.join("positive/tc-created.json");

    let result = Command::new(&binary_path)
        .arg("--create-test")
        .arg(&tc)
        .output()?;

    assert!(
        !result.status.success(),
        "Command should fail when --output is missing with --create-test"
    );

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("--output"),
        "Error message should mention --output requirement: {}",
        stderr
    );

    println!("✓ --create-test missing --output test passed");

    Ok(())
}
