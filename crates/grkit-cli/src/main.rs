//! Native command-line adapter. Formats and package rules live in grkit-core.

mod storage;

use anyhow::{ensure, Context, Result};
use clap::{Parser, Subcommand};
use grkit_core::{effects, fit, packaging, site, Files};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Room for a 256-point 3D table written with generous precision.
const MAX_CUBE_SIZE: usize = 1024 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "grkit", version, about = "Build GRKit plugins and SD effects")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the local ARM toolchain.
    Doctor {
        #[arg(long, default_value = "clang")]
        clang: String,
        #[arg(long, default_value = "ld.lld")]
        lld: String,
    },
    /// Build the production runtime from official firmware.
    Build {
        #[arg(long, value_parser = ["gr3-2.10"])]
        target: String,
        #[arg(long)]
        firmware: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "clang")]
        clang: String,
        #[arg(long, default_value = "ld.lld")]
        lld: String,
    },
    /// Compile effect definitions.
    Effects {
        #[command(subcommand)]
        command: EffectsCommand,
    },
    /// Package a verified local runtime and effects.
    Package {
        #[arg(long)]
        build: PathBuf,
        #[arg(long)]
        effects: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Check package layout, profiles and checksums.
    Verify { archive: PathBuf },
    /// Write a verified runtime build into a new static site directory.
    Site {
        #[arg(long)]
        build: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum EffectsCommand {
    Build {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write effects, preview tables, and their catalog into a new site directory.
    Publish {
        #[arg(long)]
        source: PathBuf,
        /// Official GR III 2.10 firmware, the source of the Standard base gamma.
        #[arg(long)]
        firmware: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Fit a .cube LUT into a new effect definition at OUT/ID/effect.json.
    Import {
        #[arg(long)]
        cube: PathBuf,
        /// Official GR III 2.10 firmware, the source of the Standard base gamma.
        #[arg(long)]
        firmware: PathBuf,
        #[arg(long, value_parser = ["srgb", "linear", "vlog"])]
        input: String,
        /// Exposure offset in EV, applied to scene-linear light before the LUT.
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        exposure: f64,
        /// Fit no post-curve matrix.
        #[arg(long)]
        no_post: bool,
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        label: String,
        #[arg(long, value_parser = ["film", "filter", "plain"])]
        shape: String,
        #[arg(long)]
        out: PathBuf,
    },
}

fn run(command: Command) -> Result<Value> {
    match command {
        Command::Doctor { clang, lld } => {
            let tools = grkit_build::Toolchain::discover(&clang, &lld)?;
            Ok(json!({"toolchain": tools.manifest(), "grkit_version": env!("CARGO_PKG_VERSION")}))
        }
        Command::Build {
            firmware,
            out,
            clang,
            lld,
            ..
        } => {
            let result = grkit_build::build_runtime(&firmware, &out, &clang, &lld)?;
            Ok(
                json!({"output": result.output, "target": result.manifest["target"], "files": result.files.len()}),
            )
        }
        Command::Effects {
            command: EffectsCommand::Build { source, out },
        } => {
            let compiled = storage::compile_directory(&source)?;
            storage::validate_output(&out)?;
            storage::write_directory(&out, &compiled)?;
            Ok(json!({"output": out, "effects": compiled.len()}))
        }
        Command::Effects {
            command:
                EffectsCommand::Publish {
                    source,
                    firmware,
                    out,
                },
        } => {
            let definitions = storage::read_definitions(&source)?;
            storage::validate_output(&out)?;
            let image = grkit_build::verify_firmware(&firmware)?;
            let base_gamma = grkit_build::firmware::standard_base_gamma(&image)?;
            let files = site::publish_effects(&definitions, &base_gamma)?;
            storage::write_directory(&out, &files)?;
            Ok(json!({"output": out, "target": site::TARGET, "effects": definitions.len()}))
        }
        Command::Effects {
            command:
                EffectsCommand::Import {
                    cube,
                    firmware,
                    input,
                    exposure,
                    no_post,
                    id,
                    name,
                    label,
                    shape,
                    out,
                },
        } => {
            ensure!(effects::valid_id(&id), "Invalid effect ID");
            let output = out.join(&id);
            storage::validate_output(&output)?;
            let image = grkit_build::verify_firmware(&firmware)?;
            let base_gamma = grkit_build::firmware::standard_base_gamma(&image)?;
            let bytes = storage::read_bounded(&cube, MAX_CUBE_SIZE)?;
            let request = fit::Import {
                effect_id: &id,
                name: &name,
                label: &label,
                shape: &shape,
                source: cube
                    .file_name()
                    .and_then(|name| name.to_str())
                    .context("Expected a UTF-8 .cube filename")?,
                input: input.parse()?,
                exposure,
                options: fit::Options {
                    post: !no_post,
                    ..fit::Options::default()
                },
            };
            let (result, text) = fit::import(&bytes, &base_gamma, &request)?;
            let errors = effects::parse_json(text.as_bytes())?["attribution"]["fit"].take();
            storage::write_directory(
                &output,
                &Files::from([("effect.json".to_owned(), text.into_bytes())]),
            )?;
            Ok(json!({"output": output, "effect_id": id, "fit": errors,
                "post_matrix": result.post_matrix.is_some()}))
        }
        Command::Package {
            build,
            effects,
            out,
        } => {
            let (runtime, manifest) = storage::load_build(&build)?;
            let effects = storage::read_effects(&effects)?;
            let bytes = packaging::assemble(&runtime, &manifest, &effects)?;
            let verified = packaging::verify_package(&bytes)?;
            storage::write_new(&out, &bytes)?;
            Ok(json!({"path": out, "sha256": packaging::sha256(&bytes),
                "entries": verified["entries"]}))
        }
        Command::Verify { archive } => {
            // Include room for ZIP metadata while bounding the archive read itself.
            let bytes = storage::read_bounded(&archive, 64 * 1024 * 1024)?;
            let verified = packaging::verify_package(&bytes)?;
            Ok(json!({"valid": true, "target": verified["target"],
                "effects": verified["effect_ids"].as_array().map_or(0, Vec::len)}))
        }
        Command::Site { build, out } => {
            let (target, files) = storage::export_runtime(&build, &out)?;
            Ok(json!({"output": out, "target": target, "files": files}))
        }
    }
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse().command) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("JSON result is serializable")
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("grkit: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
