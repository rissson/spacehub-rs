use crate::config::Config;
use clap::Clap;
use tracing::*;

mod config;
mod folders;
mod matrix;

#[derive(Clap)]
struct Opts {
    #[clap(short, long, default_value = "config.yml")]
    config: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}
#[derive(Clap)]
enum SubCommand {
    #[clap()]
    Create(Create),
    #[clap()]
    Export(Export),
}

/// A subcommand for creating spaces
#[derive(Clap)]
struct Create;

/// A subcommand for exporting spaces
#[derive(Clap)]
struct Export;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    let opts: Opts = Opts::parse();

    info!("Loading Configs...");
    let config = Config::load(opts.config)?;

    info!("Setting up Client...");
    let client = &mut matrix::Matrix::new(config.clone()).await?;

    match opts.subcmd {
        SubCommand::Create(c) => {}
        SubCommand::Export(e) => {}
    }
    Ok(())
}
