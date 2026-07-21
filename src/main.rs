mod adb;
mod cli;
mod error;
mod qr;
mod workflow;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let exit_code = workflow::execute(cli);
    std::process::exit(exit_code);
}
