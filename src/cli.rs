use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "adb-qr",
    version,
    about = "Quickly pair an Android device over wireless ADB"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub pair: PairArgs,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(hide = true)]
    Pair(PairArgs),
    Qr(QrArgs),
}

#[derive(Args, Debug, Clone)]
pub struct PairArgs {
    #[arg(long, default_value_t = 90)]
    pub timeout: u64,

    #[arg(long)]
    pub adb_path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct QrArgs {
    #[arg(long)]
    pub svg: Option<PathBuf>,

    #[arg(long)]
    pub png: Option<PathBuf>,

    #[arg(long)]
    pub print_payload: bool,
}
