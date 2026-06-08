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

mod batch;
mod processing;
mod profile;
mod test_case;

use anyhow::{Context, Result};
use clap::Parser;
use crtool::SUPPORTED_ASSET_EXTENSIONS;
use glob::glob;
use profile::{run_rubric_evaluation, ReportFormat};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use test_case::handle_create_test;

// ─── Logger ──────────────────────────────────────────────────────────────────

/// Output manager: writes progress to stdout (unless quiet) and optionally to a log file.
pub struct Logger {
    quiet: bool,
    log_writer: Option<BufWriter<std::fs::File>>,
}

impl Logger {
    pub fn new(quiet: bool, log_path: Option<&std::path::Path>) -> Result<Self> {
        let log_writer = if let Some(path) = log_path {
            let file = std::fs::File::create(path)
                .with_context(|| format!("Failed to create log file: {}", path.display()))?;
            eprintln!("📝 Logging to: {}", path.display());
            Some(BufWriter::new(file))
        } else {
            None
        };
        Ok(Self { quiet, log_writer })
    }

    pub fn info(&mut self, msg: &str) {
        if !self.quiet {
            println!("{msg}");
        }
        if let Some(w) = &mut self.log_writer {
            let _ = writeln!(w, "{msg}");
        }
    }

    pub fn error(&mut self, msg: &str) {
        eprintln!("{msg}");
        if let Some(w) = &mut self.log_writer {
            let _ = writeln!(w, "ERROR: {msg}");
        }
    }
}

// ─── CLI definition ───────────────────────────────────────────────────────────

/// C2PA Test Tool - Create test assets, validate assets, and evaluate rubrics
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path or glob pattern for test case YAML file(s) (C2PA test case schema).
    /// Supports glob patterns (e.g., "test-cases/positive/tc-*.yaml", "test-cases/**/*.yaml").
    /// Reads all signing configuration (manifest, cert, key, algorithm, TSA URL) from each file.
    /// Use with -o to specify the output file or directory (directory required for multiple matches).
    #[arg(short = 't', long = "create-test", value_name = "PATTERN")]
    create_test: Option<String>,

    /// Validate input asset(s) against a YAML asset grammar
    /// (schema TBD; currently scaffolded — always reports success)
    #[arg(long, default_value = "false")]
    validate: bool,

    /// Path to the YAML asset grammar for validation (required with --validate)
    #[arg(long, value_name = "FILE")]
    grammar: Option<PathBuf>,

    /// Path(s) to input media asset(s). Supported: avi, avif, c2pa, dng, gif, heic, heif,
    /// jpg/jpeg, m4a, mov, mp3, mp4, pdf, png, svg, tiff, wav, webp.
    /// Supports glob patterns (e.g., "*.jpg", "images/*.png")
    #[arg(value_name = "INPUT_FILE", required = false, num_args = 0..)]
    input: Vec<String>,

    /// Path to the output file or directory
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Enable trust list validation: load the official C2PA trust list and the Content
    /// Credentials interim trust list for certificate validation
    #[arg(long, default_value = "false")]
    trust: bool,

    /// Path to the YAML asset rubric for rubric evaluation.
    /// When used alone, treats input files as crJSON indicators.
    /// Also accepts --profile as a deprecated alias.
    #[arg(long, alias = "profile", value_name = "FILE")]
    rubric: Option<PathBuf>,

    /// Output format for the rubric evaluation report (json or yaml)
    #[arg(long, value_enum, default_value_t = ReportFormat::Yaml)]
    report_format: ReportFormat,

    /// Path to a batch JSON file — runs multiple commands in sequence
    #[arg(short = 'b', long = "batch", value_name = "FILE")]
    batch: Option<PathBuf>,

    /// Suppress progress output (errors are still shown on stderr)
    #[arg(short = 'q', long = "quiet", default_value = "false")]
    quiet: bool,

    /// Write all progress output to a log file (in addition to stdout)
    #[arg(short = 'l', long = "log", value_name = "FILE")]
    log: Option<PathBuf>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Expand glob patterns and collect matching file paths.
pub fn expand_input_patterns(patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for pattern in patterns {
        let pattern_path = PathBuf::from(pattern);

        if pattern_path.exists() {
            files.push(pattern_path);
        } else {
            let matches: Vec<PathBuf> = glob(pattern)
                .context(format!("Invalid glob pattern: {}", pattern))?
                .filter_map(|entry: std::result::Result<PathBuf, glob::GlobError>| entry.ok())
                .collect();

            if matches.is_empty() {
                anyhow::bail!("No files match pattern: {}", pattern);
            }

            files.extend(matches);
        }
    }

    files.sort();
    files.dedup();

    Ok(files)
}

// ─── Core execution ───────────────────────────────────────────────────────────

/// Execute a parsed CLI command. Called from both normal mode and batch mode.
pub fn run_cli(cli: Cli, logger: &mut Logger) -> Result<()> {
    // ── Create-test mode ─────────────────────────────────────────────────────
    if let Some(test_case_pattern) = &cli.create_test {
        let output = cli
            .output
            .context("--output is required when using --create-test mode")?;

        let test_case_files = expand_input_patterns(&[test_case_pattern.clone()])
            .context("Failed to expand --create-test pattern")?;

        // Fast path: single test case, no input override
        if test_case_files.len() == 1 && cli.input.is_empty() {
            return handle_create_test(&test_case_files[0], None, &output);
        }

        let input_files = if cli.input.is_empty() {
            vec![]
        } else {
            expand_input_patterns(&cli.input).context("Failed to expand input file patterns")?
        };

        if (test_case_files.len() > 1 || input_files.len() > 1) && !output.is_dir() {
            anyhow::bail!(
                "Output must be a directory when creating test assets from multiple test cases or input files. Got: {:?}",
                output
            );
        }

        let mut success_count = 0u32;
        let mut error_count = 0u32;

        for test_case_path in &test_case_files {
            if input_files.is_empty() {
                logger.info(&format!(
                    "  📄 Processing test case: {} ...",
                    test_case_path.display()
                ));
                match handle_create_test(test_case_path, None, &output) {
                    Ok(_) => {
                        logger.info("     ✅ Done");
                        success_count += 1;
                    }
                    Err(e) => {
                        logger.error(&format!("     ❌ Error: {e:#}"));
                        error_count += 1;
                    }
                }
            } else {
                for input_file in &input_files {
                    logger.info(&format!("  📄 Processing: {} ...", input_file.display()));
                    match handle_create_test(test_case_path, Some(input_file), &output) {
                        Ok(_) => {
                            logger.info("     ✅ Done");
                            success_count += 1;
                        }
                        Err(e) => {
                            logger.error(&format!("     ❌ Error: {e:#}"));
                            error_count += 1;
                        }
                    }
                }
            }
        }

        let total = success_count + error_count;
        if total > 1 {
            logger.info(&format!(
                "\n📊 Test Asset Creation: {success_count} succeeded, {error_count} failed, {total} total"
            ));
        }

        if error_count > 0 {
            anyhow::bail!("{error_count} file(s) failed to create test asset");
        }

        return Ok(());
    }

    // All other modes require at least one input file
    if cli.input.is_empty() {
        anyhow::bail!(
            "No input files specified. Use --create-test to create a test asset from a test \
            case JSON file, or provide input file(s) for validate/profile modes."
        );
    }

    let input_files =
        expand_input_patterns(&cli.input).context("Failed to expand input file patterns")?;

    if input_files.is_empty() {
        anyhow::bail!("No input files found matching the specified pattern(s)");
    }

    for input_file in &input_files {
        if !input_file.exists() {
            anyhow::bail!("Input file does not exist: {:?}", input_file);
        }
    }

    logger.info(&format!(
        "🚀 Processing {} input file(s)",
        input_files.len()
    ));

    // ── Validate mode (scaffold) ──────────────────────────────────────────────
    if cli.validate {
        logger.info("=== Asset Validation ===");
        logger.info("  (Note: YAML grammar-based validation is not yet implemented)");

        if let Some(grammar_path) = &cli.grammar {
            logger.info(&format!("  Grammar: {}", grammar_path.display()));
        }

        for input_file in &input_files {
            let ext_ok = crtool::is_supported_asset_path(input_file);
            logger.info(&format!(
                "  📄 {} — {}",
                input_file.display(),
                if ext_ok { "supported format ✅" } else { "unsupported format ⚠️" }
            ));
        }

        logger.info("\n📊 Validation: scaffold only — all files reported as valid");
        return Ok(());
    }

    // ── Rubric evaluation mode ────────────────────────────────────────────────
    if cli.rubric.is_some() {
        let rubric_path = cli.rubric.as_ref().unwrap();

        let unsupported_for_rubric: Vec<_> = input_files
            .iter()
            .filter(|p| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                ext != "json" && !crtool::is_supported_asset_path(p)
            })
            .collect();
        if !unsupported_for_rubric.is_empty() {
            anyhow::bail!(
                "Unsupported file format(s) for rubric evaluation: {:?}",
                unsupported_for_rubric.iter().map(|p| p.as_path()).collect::<Vec<_>>()
            );
        }

        let mut success_count = 0u32;
        let mut error_count = 0u32;

        logger.info("=== Rubric Evaluation ===");

        for input_file in &input_files {
            logger.info(&format!("  📄 Processing: {} ...", input_file.display()));
            match run_rubric_evaluation(input_file, rubric_path, cli.report_format) {
                Ok(_) => {
                    logger.info("     ✅ Done");
                    success_count += 1;
                }
                Err(e) => {
                    logger.error(&format!("     ❌ Error: {e}"));
                    error_count += 1;
                }
            }
        }

        logger.info(&format!(
            "\n📊 Evaluation Summary: {success_count} succeeded, {error_count} failed, {} total",
            input_files.len()
        ));

        if error_count > 0 {
            anyhow::bail!("{error_count} file(s) failed evaluation");
        }

        return Ok(());
    }

    // Validate that input files have supported C2PA extensions for any future operations
    let unsupported: Vec<_> = input_files
        .iter()
        .filter(|p| !crtool::is_supported_asset_path(p))
        .collect();
    if !unsupported.is_empty() {
        anyhow::bail!(
            "Unsupported file format(s). The following file(s) have extensions not supported \
            by C2PA: {:?}. Supported extensions: {}.",
            unsupported.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
            SUPPORTED_ASSET_EXTENSIONS.join(", ")
        );
    }

    anyhow::bail!(
        "No operation specified. Use --create-test FILE to create a test asset, \
        --validate to validate assets, --rubric FILE to evaluate a rubric, or \
        --batch FILE to run a batch of commands."
    );
}

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut logger = Logger::new(cli.quiet, cli.log.as_deref())?;

    if let Some(batch_path) = &cli.batch.clone() {
        return batch::run_batch(batch_path, &mut logger);
    }

    run_cli(cli, &mut logger)
}
