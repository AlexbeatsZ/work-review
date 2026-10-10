use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use std::{
    future::Future,
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
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

async fn bind_when_ready<T, F, Fut>(mut bind: F, retry_delay: Duration) -> io::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = io::Result<T>>,
{
    loop {
        match bind().await {
            Err(error) if error.kind() == io::ErrorKind::AddrNotAvailable => {
                log::warn!("Hub address is not ready; waiting for the network: {error}");
                tokio::time::sleep(retry_delay).await;
            }
            result => return result,
        }
    }
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
            let listener = bind_when_ready(
                || tokio::net::TcpListener::bind(&config.bind),
                Duration::from_secs(5),
            )
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn waits_for_the_address_then_opens_a_real_listener() {
        let mut attempts = 0;
        let listener = bind_when_ready(
            || {
                attempts += 1;
                let attempt = attempts;
                async move {
                    if attempt <= 2 {
                        Err(io::Error::from(io::ErrorKind::AddrNotAvailable))
                    } else {
                        tokio::net::TcpListener::bind("127.0.0.1:0").await
                    }
                }
            },
            Duration::ZERO,
        )
        .await
        .unwrap();
        assert_eq!(attempts, 3);
        assert!(listener.local_addr().unwrap().ip().is_loopback());
    }

    #[tokio::test]
    async fn other_bind_errors_exit_without_retrying() {
        for kind in [io::ErrorKind::AddrInUse, io::ErrorKind::PermissionDenied] {
            let mut attempts = 0;
            let result: io::Result<()> = bind_when_ready(
                || {
                    attempts += 1;
                    std::future::ready(Err(io::Error::from(kind)))
                },
                Duration::ZERO,
            )
            .await;
            assert_eq!(result.unwrap_err().kind(), kind);
            assert_eq!(attempts, 1);
        }
    }
}
