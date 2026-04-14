use anyhow::Result;
use clap::Parser;
use pdydns::{config::AppConfig, powerdns::update_a_record, public_ip::fetch_public_ip};
use reqwest::Client;
use std::{path::PathBuf, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(author, version, about = "Sync the public IP to PowerDNS A records")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    let config = AppConfig::load(&cli.config)?;
    let client = Client::builder().user_agent("pdydns/0.1").build()?;

    run(client, config).await
}

async fn run(client: Client, config: AppConfig) -> Result<()> {
    let mut last_ip: Option<String> = None;
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        match fetch_public_ip(&client, &config.public_ip_url).await {
            Ok(ip) => {
                if last_ip.as_deref() != Some(ip.as_str()) {
                    info!(%ip, "public ip changed");
                    for record in &config.records {
                        match update_a_record(&client, &config, record, &ip).await {
                            Ok(()) => {
                                info!(zone = %record.normalized_zone(), name = %record.normalized_name(), "record updated")
                            }
                            Err(err) => {
                                error!(error = %err, zone = %record.normalized_zone(), name = %record.normalized_name(), "failed to update record")
                            }
                        }
                    }
                    last_ip = Some(ip);
                } else {
                    info!(%ip, "public ip unchanged");
                }
            }
            Err(err) => warn!(error = %err, "failed to fetch public ip"),
        }

        tokio::select! {
            _ = &mut shutdown => {
                info!("shutdown requested");
                break;
            }
            _ = sleep(Duration::from_secs(config.interval_seconds)) => {}
        }
    }

    Ok(())
}
