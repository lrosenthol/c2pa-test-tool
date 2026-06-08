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
use clap::ValueEnum;
use profile_evaluator_rs::{
    evaluate_files as evaluate_profile_files, serialize_report, OutputFormat as ProfileOutputFormat,
};
use std::fs;
use std::path::Path;

/// Output format for the profile evaluation report.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ReportFormat {
    Json,
    #[default]
    Yaml,
}

/// Evaluate a crJSON file (or a signed asset) against a YAML rubric and write the report.
/// The report is written alongside the input file as `<stem>-report.<ext>`.
/// If the input is a supported C2PA asset rather than a crJSON file, the crJSON is extracted
/// from the asset's embedded manifest store before evaluation.
pub fn run_rubric_evaluation(
    input_path: &Path,
    profile_path: &Path,
    format: ReportFormat,
) -> Result<()> {
    let is_asset = crtool::is_supported_asset_path(input_path);

    // When the input is an asset, extract its crJSON to a tempfile that lives for the
    // duration of evaluation. None means the input is already a crJSON file.
    let crjson_tmpfile: Option<tempfile::NamedTempFile> = if is_asset {
        println!("Running rubric evaluation...");
        println!("  Asset:  {:?}", input_path);
        println!("  Rubric: {:?}", profile_path);

        let reader = c2pa::Reader::from_context(c2pa::Context::new())
            .with_file(input_path)
            .with_context(|| format!("Failed to read C2PA manifest from {:?}", input_path))?;
        let crjson_str = reader
            .crjson_checked()
            .context("Failed to extract crJSON from asset")?;

        let mut tmp = tempfile::Builder::new()
            .suffix(".json")
            .tempfile()
            .context("Failed to create temporary file for crJSON")?;
        use std::io::Write as _;
        tmp.write_all(crjson_str.as_bytes())
            .context("Failed to write crJSON to temporary file")?;
        Some(tmp)
    } else {
        println!("Running rubric evaluation...");
        println!("  crJSON: {:?}", input_path);
        println!("  Rubric: {:?}", profile_path);
        None
    };

    let crjson_path: &Path = crjson_tmpfile
        .as_ref()
        .map(|t| t.path())
        .unwrap_or(input_path);

    let output_format = match format {
        ReportFormat::Json => ProfileOutputFormat::Json,
        ReportFormat::Yaml => ProfileOutputFormat::Yaml,
    };

    let report = evaluate_profile_files(profile_path, crjson_path)
        .context("Failed to evaluate profile against crJSON")?;

    let serialized = serialize_report(&report, output_format)
        .context("Failed to serialize evaluation report")?;

    let stem = input_path
        .file_stem()
        .context("Input path has no filename")?
        .to_str()
        .context("Invalid UTF-8 in input filename")?;

    let report_ext = match format {
        ReportFormat::Json => "json",
        ReportFormat::Yaml => "yaml",
    };
    let report_filename = format!("{}-report.{}", stem, report_ext);
    let report_path = input_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&report_filename);

    fs::write(&report_path, serialized).context("Failed to write evaluation report")?;

    println!("✓ Rubric evaluation complete");
    println!("  Report: {:?}", report_path);

    Ok(())
}
