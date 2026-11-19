//! `pdx` — PrimeX D.T host CLI. Mirrors the in-app command bus.
//!
//! Every subcommand supports `--json` and `--quiet`. Typed exit codes:
//! 0 ok, 2 bad input, 3 unsupported format, 4 verification failure,
//! 5 capability failure, 6 cancelled (1 for unmapped failures).

use clap::{Parser, Subcommand};
use pdx_core::{PdxError, Result, MIUIX_VERSION, TOOL_VERSION};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "pdx",
    version,
    about = "PrimeX D.T — offline reverse-engineering workbench (host CLI)"
)]
struct Cli {
    #[arg(long, global = true, help = "emit machine-readable JSON")]
    json: bool,
    #[arg(long, global = true, help = "suppress non-error output")]
    quiet: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Tool and parser identity.
    Info,
    /// APK/ZIP/OBB container operations.
    Apk {
        #[command(subcommand)]
        operation: ApkCommand,
    },
}

#[derive(Subcommand)]
enum ApkCommand {
    /// List central directory entries with sizes, CRCs and 16 KB alignment.
    List { file: PathBuf },
    /// Stream CRC-32 verification of every entry.
    Verify { file: PathBuf },
}

fn read_file(path: &PathBuf) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| PdxError::io(format!("cannot read {}: {e}", path.display())))
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Info => {
            if cli.json {
                let value = serde_json::json!({
                    "tool_version": TOOL_VERSION,
                    "miuix_version": MIUIX_VERSION,
                    "crates": ["pdx-core", "pdx-apk", "pdx-cli", "pdx-jni", "pdx-term"],
                });
                println!("{value}");
            } else if !cli.quiet {
                println!("pdx {TOOL_VERSION} (Miuix {MIUIX_VERSION})");
            }
        }
        Command::Apk { operation } => match operation {
            ApkCommand::List { file } => {
                let bytes = read_file(&file)?;
                let archive = pdx_apk::parse(&bytes)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&archive)
                            .map_err(|e| PdxError::internal(format!("serialize archive: {e}")))?
                    );
                } else if !cli.quiet {
                    for entry in &archive.entries {
                        println!(
                            "{:<48} method={:<2} size={:>10} crc={:08x} align_16k={}",
                            entry.name,
                            entry.method,
                            entry.compressed_size,
                            entry.crc32,
                            entry.is_16k_aligned()
                        );
                    }
                    println!("{} entries, zip64={}", archive.entry_count, archive.zip64);
                }
            }
            ApkCommand::Verify { file } => {
                let bytes = read_file(&file)?;
                let archive = pdx_apk::parse(&bytes)?;
                let verified = pdx_apk::verify_all(&bytes, &archive)?;
                if cli.json {
                    println!("{{\"verified_entries\":{verified},\"status\":\"ok\"}}");
                } else if !cli.quiet {
                    println!("verified {verified} entries in {}", file.display());
                }
            }
        },
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.to_json());
            ExitCode::from(error.kind.exit_code() as u8)
        }
    }
}
