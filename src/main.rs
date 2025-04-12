mod cli_args;
use std::{path::Path, str::FromStr};

use anyhow::{Context, Result};
use clap::Parser;
use cli_args::CliArgs;
use transition_tables::TransitionTable;

use fa_viz::*;

fn main() -> Result<()> {
    let args = CliArgs::parse();

    match args.mode {
        cli_args::Mode::NFA => {
            let nfa: NFA = NFA::from_str(&read_file(&args.file))
                .context("Unable to parse input file to NFA")?;
            let graph = Digraph::from(&nfa);
            println!("{}", graph);
        }
        cli_args::Mode::DFA => {
            let tt = TransitionTable::parse(&read_file(&args.file))
                .context("Unable to parse input file to DFA")?;
            let graph = Digraph::from(&tt);
            println!("{}", graph);
        }
    }

    Ok(())
}

fn read_file(p: &Path) -> String {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).expect("Unable to open file!");
    let mut res = String::new();
    file.read_to_string(&mut res)
        .expect("Unable to read file contents to string!");

    res
}
