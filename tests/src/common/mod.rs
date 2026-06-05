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

// Shared test helpers are used by different test binaries; not every binary uses every helper.
#![allow(dead_code)]

use anyhow::Result;
use c2pa::{Builder, CallbackSigner, Context, Ingredient, Reader, Relationship, SigningAlg};
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Path to the c2pa-test-tool CLI binary.
pub fn cli_binary_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_C2PA_TEST_TOOL") {
        return PathBuf::from(path);
    }
    // Fallback: locate by convention relative to manifest dir
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let exe = if cfg!(windows) {
        "c2pa-test-tool.exe"
    } else {
        "c2pa-test-tool"
    };
    manifest_dir.join("target").join(profile).join(exe)
}

/// Test helper to get the path to test fixtures
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Test helper to get the path to test certificates
pub fn certs_dir() -> PathBuf {
    fixtures_dir().join("certs")
}

/// Test helper to get the path to raw (unsigned) test images
pub fn testfiles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assets/raw")
}

/// Test helper to get the path to pre-signed test assets
pub fn signed_assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assets/signed")
}

/// Test helper to get the path to testset
pub fn testset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testset")
}

/// Test helper to create output directory for test artifacts
pub fn output_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_output");
    fs::create_dir_all(&dir).expect("Failed to create test output directory");
    dir
}

/// Collect resource identifiers (icon, thumbnail, data) from manifest JSON so we can load
/// them from the manifest's directory and add to the builder before signing.
fn collect_manifest_resource_identifiers(manifest: &serde_json::Value) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(cgi) = manifest.get("claim_generator_info") {
        let entries: Vec<&serde_json::Value> = cgi
            .as_array()
            .map(|a| a.iter().collect())
            .unwrap_or_else(|| vec![cgi]);
        for entry in entries {
            if let Some(icon) = entry.get("icon").and_then(|i| i.get("identifier")) {
                if let Some(s) = icon.as_str() {
                    ids.insert(s.to_string());
                }
            }
        }
    }
    if let Some(t) = manifest.get("thumbnail").and_then(|t| t.get("identifier")) {
        if let Some(s) = t.as_str() {
            ids.insert(s.to_string());
        }
    }
    if let Some(ingredients) = manifest.get("ingredients").and_then(|v| v.as_array()) {
        for ing in ingredients {
            if let Some(t) = ing.get("thumbnail").and_then(|t| t.get("identifier")) {
                if let Some(s) = t.as_str() {
                    ids.insert(s.to_string());
                }
            }
            if let Some(d) = ing.get("data").and_then(|d| d.get("identifier")) {
                if let Some(s) = d.as_str() {
                    ids.insert(s.to_string());
                }
            }
        }
    }
    if let Some(assertions) = manifest.get("assertions").and_then(|v| v.as_array()) {
        for assertion in assertions {
            let data = assertion.get("data").and_then(|d| d.as_object());
            if let Some(templates) = data
                .and_then(|d| d.get("templates"))
                .and_then(|t| t.as_array())
            {
                for template in templates {
                    if let Some(icon) = template.get("icon").and_then(|i| i.get("identifier")) {
                        if let Some(s) = icon.as_str() {
                            ids.insert(s.to_string());
                        }
                    }
                }
            }
        }
    }
    ids
}

/// Load manifest-referenced resources (icons, thumbnails, etc.) from a base directory
/// and add them to the builder.
fn add_manifest_resources_from_dir(
    builder: &mut Builder,
    manifest_json: &str,
    base_dir: &Path,
) -> Result<()> {
    let manifest: serde_json::Value = serde_json::from_str(manifest_json)?;
    let identifiers = collect_manifest_resource_identifiers(&manifest);
    for id in identifiers {
        let path = base_dir.join(&id);
        if path.exists() && path.is_file() {
            let data = fs::read(&path)?;
            builder.add_resource(&id, Cursor::new(data))?;
        }
    }
    Ok(())
}

/// If the JSON is a test-case file (has a top-level `"manifest"` key), return the
/// serialized content of that key. Otherwise return the original JSON unchanged.
fn extract_manifest_json(json: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(manifest) = val.get("manifest") {
            if let Ok(s) = serde_json::to_string(manifest) {
                return s;
            }
        }
    }
    json.to_string()
}

/// Helper function to sign a file with a manifest that includes file-based ingredients
pub fn sign_file_with_manifest_and_ingredients(
    input_path: &Path,
    output_path: &Path,
    manifest_path: &Path,
    ingredients_base_dir: &Path,
) -> Result<()> {
    sign_file_with_manifest_and_ingredients_impl(
        input_path,
        output_path,
        manifest_path,
        ingredients_base_dir,
        false,
        false,
    )
}

fn sign_file_with_manifest_and_ingredients_impl(
    input_path: &Path,
    output_path: &Path,
    manifest_path: &Path,
    ingredients_base_dir: &Path,
    generate_asset_thumbnail: bool,
    generate_ingredient_thumbnails: bool,
) -> Result<()> {
    use std::io::Cursor;
    if output_path.exists() {
        fs::remove_file(output_path)?;
    }

    let raw_json = fs::read_to_string(manifest_path)?;
    let manifest_json = extract_manifest_json(&raw_json);

    let (file_ingredients, cleaned_manifest) = process_ingredients_with_thumbnails(
        &manifest_json,
        ingredients_base_dir,
        generate_ingredient_thumbnails,
    )?;

    let mut builder = Builder::from_context(Context::new()).with_definition(&cleaned_manifest)?;

    let manifest_base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    add_manifest_resources_from_dir(&mut builder, &manifest_json, manifest_base_dir)?;

    for ingredient in file_ingredients {
        builder.add_ingredient(ingredient);
    }

    if generate_asset_thumbnail {
        let mut input_file = fs::File::open(input_path)?;

        let input_extension = input_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Input file has no extension"))?;

        let input_format = extension_to_mime(input_extension)
            .ok_or_else(|| anyhow::anyhow!("Unsupported input file format"))?;

        let (thumb_format, thumbnail) = make_thumbnail_from_stream(input_format, &mut input_file)?;
        builder.set_thumbnail(&thumb_format, &mut Cursor::new(thumbnail))?;
    }

    let signer = test_signer();
    builder.sign_file(&signer, input_path, output_path)?;

    Ok(())
}

fn process_ingredients_with_thumbnails(
    manifest_json: &str,
    ingredients_base_dir: &Path,
    generate_thumbnails: bool,
) -> Result<(Vec<Ingredient>, String)> {
    use serde_json::Value as JsonValue;
    use std::io::Seek;

    let mut manifest: JsonValue = serde_json::from_str(manifest_json)?;
    let mut file_ingredients: Vec<Ingredient> = Vec::new();

    if let Some(ingredients) = manifest
        .get("ingredients")
        .and_then(|v| v.as_array())
        .cloned()
    {
        let mut inline_ingredients = Vec::new();

        for ingredient_def in &ingredients {
            let Some(file_path_str) = ingredient_def.get("file_path").and_then(|v| v.as_str())
            else {
                inline_ingredients.push(ingredient_def.clone());
                continue;
            };

            let file_path = if Path::new(file_path_str).is_absolute() {
                PathBuf::from(file_path_str)
            } else {
                ingredients_base_dir.join(file_path_str)
            };

            if !file_path.exists() {
                anyhow::bail!("Ingredient file not found: {:?}", file_path);
            }

            let mut source = fs::File::open(&file_path)?;

            let extension = file_path
                .extension()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Ingredient file has no extension"))?;

            let format = extension_to_mime(extension)
                .ok_or_else(|| anyhow::anyhow!("Unsupported ingredient file format"))?;

            #[allow(deprecated)]
            let mut ingredient = Ingredient::from_stream(format, &mut source)?;

            if let Some(title) = ingredient_def.get("title").and_then(|v| v.as_str()) {
                ingredient.set_title(title);
            }

            if let Some(rel) = ingredient_def.get("relationship").and_then(|v| v.as_str()) {
                let relationship = match rel.to_lowercase().as_str() {
                    "parentof" => Relationship::ParentOf,
                    "componentof" => Relationship::ComponentOf,
                    _ => anyhow::bail!("Invalid relationship type: {}", rel),
                };
                ingredient.set_relationship(relationship);
            }

            if let Some(label) = ingredient_def.get("label").and_then(|v| v.as_str()) {
                ingredient.set_instance_id(label);
            }

            if let Some(metadata_obj) = ingredient_def.get("metadata") {
                if let Some(metadata_map) = metadata_obj.as_object() {
                    use c2pa::assertions::AssertionMetadata;
                    let mut assertion_metadata = AssertionMetadata::new();
                    for (key, value) in metadata_map {
                        assertion_metadata = assertion_metadata.set_field(key, value.clone());
                    }
                    ingredient.set_metadata(assertion_metadata);
                }
            }

            if generate_thumbnails && ingredient.thumbnail_ref().is_none() {
                source.rewind()?;
                let (thumb_format, thumbnail) = make_thumbnail_from_stream(format, &mut source)?;
                ingredient.set_thumbnail(&thumb_format, thumbnail)?;
            }

            file_ingredients.push(ingredient);
        }

        if let Some(obj) = manifest.as_object_mut() {
            obj.insert(
                "ingredients".to_string(),
                JsonValue::Array(inline_ingredients),
            );
        }
    }

    let cleaned_json = serde_json::to_string(&manifest)?;
    Ok((file_ingredients, cleaned_json))
}

fn extension_to_mime(extension: &str) -> Option<&'static str> {
    Some(match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "tiff" | "tif" => "image/tiff",
        "bmp" => "image/bmp",
        _ => return None,
    })
}

fn make_thumbnail_from_stream(format: &str, stream: &mut fs::File) -> Result<(String, Vec<u8>)> {
    use image::ImageFormat;
    use std::io::{BufReader, Cursor};

    let img_format = match format {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        "image/gif" => ImageFormat::Gif,
        "image/bmp" => ImageFormat::Bmp,
        "image/tiff" => ImageFormat::Tiff,
        "image/webp" => ImageFormat::WebP,
        _ => ImageFormat::Jpeg,
    };

    let reader = BufReader::new(stream);
    let img = image::load(reader, img_format)?;

    const THUMBNAIL_SIZE: u32 = 256;
    let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

    let mut buf = Cursor::new(Vec::new());
    thumbnail.write_to(&mut buf, ImageFormat::Jpeg)?;

    Ok(("image/jpeg".to_string(), buf.into_inner()))
}

fn test_signer() -> CallbackSigner {
    const CERTS: &[u8] = include_bytes!("../../fixtures/certs/ed25519.pub");
    const PRIVATE_KEY: &[u8] = include_bytes!("../../fixtures/certs/ed25519.pem");

    let ed_signer = |_context: *const (), data: &[u8]| ed_sign(data, PRIVATE_KEY);
    CallbackSigner::new(ed_signer, SigningAlg::Ed25519, CERTS)
        .set_context("test" as *const _ as *const ())
}

fn ed_sign(data: &[u8], private_key: &[u8]) -> c2pa::Result<Vec<u8>> {
    use c2pa::crypto::raw_signature::RawSignerError;
    use ed25519_dalek::{Signature, Signer, SigningKey};
    use pem::parse;

    let pem = parse(private_key).map_err(|e| c2pa::Error::OtherError(Box::new(e)))?;
    let key_bytes = &pem.contents()[16..];
    let signing_key = SigningKey::try_from(key_bytes)
        .map_err(|e| RawSignerError::InternalError(e.to_string()))?;
    let signature: Signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Helper function to verify a signed file has a valid manifest
pub fn verify_signed_file(file_path: &Path) -> Result<Reader> {
    let reader = Reader::from_context(Context::new()).with_file(file_path)?;

    assert!(
        reader.active_label().is_some(),
        "File should have an active manifest"
    );

    Ok(reader)
}

/// Helper to get all test image files
pub fn get_test_images() -> Vec<PathBuf> {
    let testfiles = testfiles_dir();
    vec![
        testfiles.join("Dog.jpg"),
        testfiles.join("Dog.png"),
        testfiles.join("Dog.webp"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_exist() {
        assert!(testfiles_dir().exists(), "testfiles directory should exist");
        assert!(certs_dir().exists(), "test certs directory should exist");
    }

    #[test]
    fn test_images_exist() {
        for img in get_test_images() {
            assert!(img.exists(), "Test image should exist: {:?}", img);
        }
    }

    #[test]
    fn test_certs_exist() {
        let cert = certs_dir().join("es256_cert.pem");
        let key = certs_dir().join("es256_private.pem");
        assert!(cert.exists(), "Test certificate should exist");
        assert!(key.exists(), "Test private key should exist");
    }
}
