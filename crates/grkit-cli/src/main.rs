//! Native command-line adapter. Formats and package rules live in grkit-core.

mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand};
use grkit_core::packaging;
use serde_json::{json, Value};
use std::path::PathBuf;

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
}

#[derive(Subcommand)]
enum EffectsCommand {
    Build {
        #[arg(long)]
        source: PathBuf,
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
