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
    #[default]
    Json,
    Yaml,
}

/// Evaluate a crJSON file against a YAML asset profile and write the report.
/// The report is written alongside the crJSON file as `<stem>-report.<ext>`.
pub fn run_profile_evaluation(
    crjson_path: &Path,
    profile_path: &Path,
    format: ReportFormat,
) -> Result<()> {
    println!("Running profile evaluation...");
    println!("  crJSON: {:?}", crjson_path);
    println!("  Profile: {:?}", profile_path);

    let output_format = match format {
        ReportFormat::Json => ProfileOutputFormat::Json,
        ReportFormat::Yaml => ProfileOutputFormat::Yaml,
    };

    let report = evaluate_profile_files(profile_path, crjson_path)
        .context("Failed to evaluate profile against crJSON")?;

    let serialized = serialize_report(&report, output_format)
        .context("Failed to serialize evaluation report")?;

    let ext = match format {
        ReportFormat::Json => "json",
        ReportFormat::Yaml => "yaml",
    };

    let stem = crjson_path
        .file_stem()
        .context("crJSON path has no filename")?
        .to_str()
        .context("Invalid UTF-8 in crJSON filename")?;

    let report_filename = format!("{}-report.{}", stem, ext);
    let report_path = crjson_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&report_filename);

    fs::write(&report_path, serialized).context("Failed to write evaluation report")?;

    println!("✓ Profile evaluation complete");
    println!("  Report: {:?}", report_path);

    Ok(())
}
