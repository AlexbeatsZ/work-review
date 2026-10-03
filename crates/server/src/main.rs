use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use work_review_core::{
    config::save_json,
    database::{lock_directory, Store},
};
use work_review_server::{Hub, ServerConfig};

#[derive(Parser)]
#[command(version, about = "Work Review central dashboard and ingest hub")]
struct Cli {
    #[arg(long, global = true, default_value = "data/hub")]
    data_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, default_value = "127.0.0.1:47831")]
        bind: String,
    },
    Run,
    Credentials,
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    let path = cli.data_dir.join("config.json");
    match cli.command {
        Command::Init { bind } => {
            let _lock = lock_directory(&cli.data_dir, "config.lock")?;
            ensure!(!path.exists(), "hub config already exists");
            let config = ServerConfig {
                bind,
                ..Default::default()
            };
            config.validate()?;
            save_json(&path, &config)?;
            println!(
                "Initialized {}; use credentials to view connection keys",
                path.display()
            );
        }
        Command::Credentials => println!("{}", std::fs::read_to_string(path)?),
        Command::Run => {
            let _lock = lock_directory(&cli.data_dir, "hub.lock")?;
            let mut config: ServerConfig = serde_json::from_slice(&std::fs::read(&path)?)?;
            if let Ok(value) = std::env::var("WORK_REVIEW_AGENT_TOKEN") {
                config.agent_token = value;
            }
            if let Ok(value) = std::env::var("WORK_REVIEW_VIEW_TOKEN") {
                config.view_token = value;
            }
            config.validate()?;
            let listener = tokio::net::TcpListener::bind(&config.bind).await?;
            println!("Work Review listening on {}", listener.local_addr()?);
            let hub = Arc::new(Hub {
                store: Mutex::new(Store::open(&cli.data_dir.join("hub.db"))?),
                directory: cli.data_dir,
                config,
            });
            axum::serve(listener, work_review_server::router(hub))
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
        }
    }
    Ok(())
}
