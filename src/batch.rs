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

use super::{run_cli, Cli, Logger};
use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct BatchCommand {
    command: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default, rename = "inputFiles")]
    input_files: Vec<String>,
}

/// Execute a batch file: parse the JSON array and run each command in sequence.
pub fn run_batch(batch_path: &Path, logger: &mut Logger) -> Result<()> {
    let content = std::fs::read_to_string(batch_path)
        .with_context(|| format!("Failed to read batch file: {}", batch_path.display()))?;
    let commands: Vec<BatchCommand> =
        serde_json::from_str(&content).context("Failed to parse batch file as JSON")?;

    let total = commands.len();
    logger.info(&format!(
        "📋 Batch: {} command(s) from {}",
        total,
        batch_path.display()
    ));

    let mut succeeded = 0u32;
    let mut failed = 0u32;

    for (i, cmd) in commands.iter().enumerate() {
        let idx = i + 1;
        let file_count = cmd.input_files.len();
        logger.info(&format!(
            "\n🔄 [{idx}/{total}] {} — {file_count} file(s)",
            cmd.command
        ));

        // Build synthetic argv: binary name + input files + extra arguments
        let mut argv = vec!["c2pa-test-tool".to_string()];
        argv.extend(cmd.input_files.clone());
        argv.extend(cmd.arguments.clone());

        match Cli::try_parse_from(&argv) {
            Ok(cli) => match run_cli(cli, logger) {
                Ok(_) => {
                    logger.info(&format!("✅ Command [{idx}/{total}] complete"));
                    succeeded += 1;
                }
                Err(e) => {
                    logger.error(&format!("❌ Command [{idx}/{total}] failed: {e}"));
                    failed += 1;
                }
            },
            Err(e) => {
                logger.error(&format!(
                    "❌ Command [{idx}/{total}] invalid arguments: {e}"
                ));
                failed += 1;
            }
        }
    }

    let fail_note = if failed > 0 {
        format!(", {failed} failed ❌")
    } else {
        String::new()
    };
    logger.info(&format!(
        "\n📊 Batch complete: {succeeded}/{total} commands succeeded ✅{fail_note}"
    ));

    if failed > 0 {
        anyhow::bail!("{failed} command(s) failed");
    }

    Ok(())
}
