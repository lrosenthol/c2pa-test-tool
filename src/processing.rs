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

use anyhow::{Context, Result};
use c2pa::{create_signer, Builder, CallbackSigner, Ingredient, Relationship, SigningAlg};
use serde_json::Value as JsonValue;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

/// Configuration for processing files with C2PA manifests
pub struct ProcessingConfig<'a> {
    pub manifest_json: &'a str,
    pub ingredients_base_dir: &'a Path,
    pub cert: &'a Path,
    pub key: &'a Path,
    pub signing_alg: SigningAlg,
    pub tsa_url: Option<String>,
    pub allow_self_signed: bool,
}

fn determine_output_path(input: &Path, output: &Path) -> Result<PathBuf> {
    if output.is_dir() {
        let filename = input.file_name().context("Input file has no filename")?;
        Ok(output.join(filename))
    } else {
        Ok(output.to_path_buf())
    }
}

/// Converts a file extension to a MIME type
fn extension_to_mime(extension: &str) -> Option<&'static str> {
    Some(match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "psd" => "image/vnd.adobe.photoshop",
        "tiff" | "tif" => "image/tiff",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "dng" => "image/x-adobe-dng",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "avif" => "image/avif",
        "avi" => "video/avi",
        "c2pa" => "application/c2pa",
        "mp2" | "mpa" | "mpe" | "mpeg" | "mpg" | "mpv2" => "video/mpeg",
        "mp4" => "video/mp4",
        "mov" | "qt" => "video/quicktime",
        "m4a" => "audio/mp4",
        "mid" | "rmi" => "audio/mid",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "aif" | "aifc" | "aiff" => "audio/aiff",
        "ogg" => "audio/ogg",
        "pdf" => "application/pdf",
        "ai" => "application/postscript",
        _ => return None,
    })
}

/// Generate a thumbnail from an image stream.
/// Returns (format, thumbnail_bytes).
fn make_thumbnail_from_stream(format: &str, stream: &mut fs::File) -> Result<(String, Vec<u8>)> {
    use image::ImageFormat;

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
    let img =
        image::load(reader, img_format).context("Failed to load image for thumbnail generation")?;

    const THUMBNAIL_SIZE: u32 = 256;
    let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

    let mut buf = Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut buf, ImageFormat::Jpeg)
        .context("Failed to encode thumbnail")?;

    Ok(("image/jpeg".to_string(), buf.into_inner()))
}

/// Load a C2PA ingredient from a file, optionally generating a thumbnail.
fn load_ingredient_from_file(file_path: &Path, generate_thumbnail: bool) -> Result<Ingredient> {
    if !file_path.exists() {
        anyhow::bail!("Ingredient file not found: {:?}", file_path);
    }

    println!("  Loading ingredient: {:?}", file_path);

    let mut source = fs::File::open(file_path)
        .context(format!("Failed to open ingredient file: {:?}", file_path))?;

    let extension = file_path
        .extension()
        .and_then(|s| s.to_str())
        .context(format!("Ingredient file has no extension: {:?}", file_path))?;

    let format = extension_to_mime(extension)
        .context(format!("Unsupported ingredient file format: {}", extension))?;

    #[allow(deprecated)]
    let mut ingredient = Ingredient::from_stream(format, &mut source).context(format!(
        "Failed to create ingredient from file: {:?}",
        file_path
    ))?;

    if generate_thumbnail && ingredient.thumbnail_ref().is_none() {
        use std::io::Seek;
        source.rewind()?;
        let (thumb_format, thumbnail) = make_thumbnail_from_stream(format, &mut source)
            .context("Failed to generate thumbnail for ingredient")?;
        ingredient
            .set_thumbnail(&thumb_format, thumbnail)
            .context("Failed to set thumbnail for ingredient")?;
    }

    Ok(ingredient)
}

/// Process file-based ingredient entries from the `ingredients` array in the manifest JSON.
/// Entries with a `file_path` field are loaded from disk and returned as `Ingredient` objects.
/// Also returns the manifest JSON with file-based entries stripped from `ingredients`, so the
/// result is safe to pass to `Builder::from_json` without conflicts.
pub fn process_ingredients(
    manifest_json: &str,
    ingredients_base_dir: &Path,
    generate_thumbnails: bool,
) -> Result<(Vec<Ingredient>, String)> {
    let mut manifest: JsonValue =
        serde_json::from_str(manifest_json).context("Failed to parse manifest JSON")?;

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

            let mut ingredient = load_ingredient_from_file(&file_path, generate_thumbnails)?;

            if let Some(title) = ingredient_def.get("title").and_then(|v| v.as_str()) {
                ingredient.set_title(title);
            } else {
                let filename = file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown");
                ingredient.set_title(filename);
            }

            if let Some(rel) = ingredient_def.get("relationship").and_then(|v| v.as_str()) {
                let relationship = match rel.to_lowercase().as_str() {
                    "parentof" => Relationship::ParentOf,
                    "componentof" => Relationship::ComponentOf,
                    _ => {
                        anyhow::bail!("Invalid relationship type: {}", rel);
                    }
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
                    println!(
                        "  Set {} metadata field(s) on ingredient",
                        metadata_map.len()
                    );
                }
            }

            file_ingredients.push(ingredient);
        }

        // Replace ingredients array with only the inline (non-file-based) entries
        if let Some(obj) = manifest.as_object_mut() {
            obj.insert(
                "ingredients".to_string(),
                JsonValue::Array(inline_ingredients),
            );
        }
    }

    let cleaned_json =
        serde_json::to_string(&manifest).context("Failed to serialize cleaned manifest JSON")?;

    Ok((file_ingredients, cleaned_json))
}

/// Parse a signing algorithm name string (case-insensitive) into a `SigningAlg`.
pub fn parse_signing_algorithm(alg: &str) -> Result<SigningAlg> {
    match alg.to_lowercase().as_str() {
        "es256" => Ok(SigningAlg::Es256),
        "es384" => Ok(SigningAlg::Es384),
        "es512" => Ok(SigningAlg::Es512),
        "ps256" => Ok(SigningAlg::Ps256),
        "ps384" => Ok(SigningAlg::Ps384),
        "ps512" => Ok(SigningAlg::Ps512),
        "ed25519" => Ok(SigningAlg::Ed25519),
        _ => anyhow::bail!("Unsupported signing algorithm: {}", alg),
    }
}

/// Detect the signing algorithm from a certificate file by examining its public key OID.
pub fn detect_signing_algorithm(cert_path: &Path) -> Result<SigningAlg> {
    use x509_parser::prelude::*;

    let cert_data = fs::read(cert_path).context("Failed to read certificate file")?;

    let pem = ::pem::parse(&cert_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse certificate PEM: {}", e))?;

    let (_, cert) = X509Certificate::from_der(pem.contents())
        .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;

    let public_key = cert.public_key();
    let alg_oid = &public_key.algorithm.algorithm;

    match alg_oid.to_id_string().as_str() {
        "1.2.840.10045.2.1" => {
            if let Some(params) = &public_key.algorithm.parameters {
                let curve_oid = params
                    .as_oid()
                    .map_err(|_| anyhow::anyhow!("Failed to parse curve OID"))?;

                match curve_oid.to_id_string().as_str() {
                    "1.2.840.10045.3.1.7" => Ok(SigningAlg::Es256),
                    "1.3.132.0.34" => Ok(SigningAlg::Es384),
                    "1.3.132.0.35" => Ok(SigningAlg::Es512),
                    other => anyhow::bail!("Unsupported EC curve OID: {}", other),
                }
            } else {
                anyhow::bail!("EC key missing curve parameters")
            }
        }
        "1.2.840.113549.1.1.1" => Ok(SigningAlg::Ps256),
        "1.3.101.112" => Ok(SigningAlg::Ed25519),
        other => anyhow::bail!("Unsupported public key algorithm OID: {}", other),
    }
}

/// Create a `CallbackSigner` that bypasses certificate chain validation.
/// Used for development and test certificates that are self-signed.
fn create_callback_signer(
    cert_path: &Path,
    key_path: &Path,
    signing_alg: SigningAlg,
) -> Result<CallbackSigner> {
    let cert_data = fs::read(cert_path).context("Failed to read certificate file")?;
    let key_data = fs::read(key_path).context("Failed to read private key file")?;

    let signer = match signing_alg {
        SigningAlg::Ed25519 => {
            let ed_signer = move |_context: *const (), data: &[u8]| ed25519_sign(data, &key_data);
            CallbackSigner::new(ed_signer, signing_alg, cert_data)
        }
        SigningAlg::Es256 | SigningAlg::Es384 | SigningAlg::Es512 => {
            let es_signer = move |_context: *const (), data: &[u8]| ecdsa_sign(data, &key_data);
            CallbackSigner::new(es_signer, signing_alg, cert_data)
        }
        SigningAlg::Ps256 | SigningAlg::Ps384 | SigningAlg::Ps512 => {
            let ps_signer = move |_context: *const (), data: &[u8]| rsa_sign(data, &key_data);
            CallbackSigner::new(ps_signer, signing_alg, cert_data)
        }
    };

    Ok(signer)
}

fn ed25519_sign(data: &[u8], private_key: &[u8]) -> c2pa::Result<Vec<u8>> {
    use c2pa::crypto::raw_signature::RawSignerError;
    use ed25519_dalek::{Signature, Signer, SigningKey};

    let pem = ::pem::parse(private_key).map_err(|e| c2pa::Error::OtherError(Box::new(e)))?;
    let key_bytes = &pem.contents()[16..];
    let signing_key = SigningKey::try_from(key_bytes)
        .map_err(|e| RawSignerError::InternalError(e.to_string()))?;
    let signature: Signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

fn ecdsa_sign(data: &[u8], private_key: &[u8]) -> c2pa::Result<Vec<u8>> {
    use c2pa::crypto::raw_signature::RawSignerError;
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};
    use p256::pkcs8::DecodePrivateKey;

    let pem = ::pem::parse(private_key).map_err(|e| c2pa::Error::OtherError(Box::new(e)))?;
    let signing_key = SigningKey::from_pkcs8_der(pem.contents())
        .map_err(|e: p256::pkcs8::Error| RawSignerError::InternalError(e.to_string()))?;
    let signature: Signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

fn rsa_sign(data: &[u8], private_key: &[u8]) -> c2pa::Result<Vec<u8>> {
    use c2pa::crypto::raw_signature::RawSignerError;
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::sha2::Sha256;
    use rsa::signature::{SignatureEncoding, Signer};
    use rsa::RsaPrivateKey;

    let pem = ::pem::parse(private_key).map_err(|e| c2pa::Error::OtherError(Box::new(e)))?;
    let private_key = RsaPrivateKey::from_pkcs8_der(pem.contents())
        .map_err(|e: rsa::pkcs8::Error| RawSignerError::InternalError(e.to_string()))?;
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(data);
    Ok(signature.to_vec())
}

/// Recursively scan a JSON value for `identifier` fields that point to local files,
/// and add those files to the builder's resource store.
fn add_manifest_resources(builder: &mut Builder, value: &JsonValue, base_dir: &Path) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            if let Some(JsonValue::String(identifier)) = map.get("identifier") {
                let resource_path = if Path::new(identifier).is_absolute() {
                    PathBuf::from(identifier)
                } else {
                    base_dir.join(identifier)
                };
                if resource_path.exists() {
                    let mut file = fs::File::open(&resource_path)
                        .context(format!("Failed to open resource file: {:?}", resource_path))?;
                    // Ignore duplicate-resource errors (resource may be referenced multiple times)
                    let _ = builder.add_resource(identifier, &mut file);
                }
            }
            for v in map.values() {
                add_manifest_resources(builder, v, base_dir)?;
            }
        }
        JsonValue::Array(arr) => {
            for item in arr {
                add_manifest_resources(builder, item, base_dir)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Sign and embed a C2PA manifest into a single asset file.
pub fn process_single_file(
    input_path: &Path,
    output_path: &Path,
    config: &ProcessingConfig,
) -> Result<()> {
    println!("\n=== Processing: {:?} ===", input_path);

    if !input_path.exists() {
        anyhow::bail!("Input file does not exist: {:?}", input_path);
    }

    let final_output_path = determine_output_path(input_path, output_path)?;

    if let Some(parent) = final_output_path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    if final_output_path.exists() {
        fs::remove_file(&final_output_path).context("Failed to remove existing output file")?;
        println!(
            "  Note: Removed existing output file: {:?}",
            final_output_path
        );
    }

    println!("  Input: {:?}", input_path);
    println!("  Output: {:?}", final_output_path);

    let (file_ingredients, cleaned_manifest) =
        process_ingredients(config.manifest_json, config.ingredients_base_dir, false)
            .context("Failed to process ingredients")?;

    let mut builder = Builder::from_context(c2pa::Context::new())
        .with_definition(cleaned_manifest.as_str())
        .context("Failed to create builder from JSON manifest")?;

    let ingredient_count = file_ingredients.len();
    for ingredient in file_ingredients {
        builder.add_ingredient(ingredient);
    }

    if ingredient_count > 0 {
        println!("  Processed {} ingredient(s) from files", ingredient_count);
    }

    let manifest_value: JsonValue = serde_json::from_str(&cleaned_manifest)
        .context("Failed to parse cleaned manifest for resource loading")?;
    add_manifest_resources(&mut builder, &manifest_value, config.ingredients_base_dir)
        .context("Failed to add manifest resources")?;

    if config.allow_self_signed {
        let signer = create_callback_signer(config.cert, config.key, config.signing_alg)
            .context("Failed to create callback signer")?;
        builder
            .sign_file(&signer, input_path, &final_output_path)
            .context("Failed to sign and embed manifest")?;
    } else {
        let signer = create_signer::from_files(
            config.cert.to_str().context("Invalid cert path")?,
            config.key.to_str().context("Invalid key path")?,
            config.signing_alg,
            config.tsa_url.clone(),
        )
        .context("Failed to create signer")?;
        builder
            .sign_file(&*signer, input_path, &final_output_path)
            .context("Failed to sign and embed manifest")?;
    }

    println!("✓ Successfully created and embedded C2PA manifest");
    println!("  Output file: {:?}", final_output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use c2pa::SigningAlg;

    #[test]
    fn test_detect_signing_algorithm_ed25519() {
        let cert_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/certs/ed25519.pub");

        if cert_path.exists() {
            let result = detect_signing_algorithm(&cert_path);
            assert!(
                result.is_ok(),
                "Should successfully detect Ed25519 algorithm"
            );
            assert_eq!(result.unwrap(), SigningAlg::Ed25519);
        }
    }

    #[test]
    fn test_parse_signing_algorithm() {
        assert_eq!(parse_signing_algorithm("es256").unwrap(), SigningAlg::Es256);
        assert_eq!(parse_signing_algorithm("ES256").unwrap(), SigningAlg::Es256);
        assert_eq!(parse_signing_algorithm("es384").unwrap(), SigningAlg::Es384);
        assert_eq!(parse_signing_algorithm("es512").unwrap(), SigningAlg::Es512);
        assert_eq!(parse_signing_algorithm("ps256").unwrap(), SigningAlg::Ps256);
        assert_eq!(parse_signing_algorithm("ps384").unwrap(), SigningAlg::Ps384);
        assert_eq!(parse_signing_algorithm("ps512").unwrap(), SigningAlg::Ps512);
        assert_eq!(
            parse_signing_algorithm("ed25519").unwrap(),
            SigningAlg::Ed25519
        );
        assert!(parse_signing_algorithm("invalid").is_err());
    }
}
