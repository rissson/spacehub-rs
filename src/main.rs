use crate::config::Config;
use clap::Parser;
use color_eyre::eyre::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use tempdir::TempDir;
use tracing::*;

mod config;
mod folders;
mod ldap;
mod matrix;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Opts {
    #[clap(short, long, parse(from_os_str))]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .json()
        .init();

    let opts = Opts::parse();

    info!("Loading config...");
    let config = Config::load(opts.config)?;

    let matrix_client = &mut matrix::MatrixClient::new(&config.matrix).await?;
    let mut ldap_client = ldap::LdapClient::new(&config.ldap).await?;

    info!("Cloning {}...", config.git_repository);
    let git_path = TempDir::new("spacehub")?.path().join("git");
    let _ = git2::build::RepoBuilder::new()
        .bare(false)
        .clone(&config.git_repository, &git_path)?;

    let mut space_folders = folders::SpaceFolder::new(git_path.as_path())?;

    for folder in &space_folders {
        folder.check()?;
    }

    for folder in &mut space_folders {
        folder
            .populate_rooms_users(
                &mut ldap_client,
                &config.ldap.localpart_template,
                &config.matrix.server_name,
                config.ldap.synapse_external_ids.as_ref(),
            )
            .await?;
    }

    if config.ldap.create_missing_users {
        info!("Creating missing users.");
        let users = space_folders
            .iter()
            .fold(HashSet::new(), |mut acc, folder| {
                acc.extend(folder.get_all_users());
                acc
            });

        for user in users {
            matrix_client.ensure_user(&user).await?;
        }
    }

    info!("Processing spaces and rooms.");
    for folder in space_folders {
        folder.folders_to_matrix(matrix_client, None).await?;
    }

    Ok(())
}
