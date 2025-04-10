use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct CliArgs {
    #[command(subcommand)]
    pub mode: Mode,
    pub file: PathBuf,
}

#[derive(Subcommand)]
pub enum Mode {
    NFA,
    DFA,
}
