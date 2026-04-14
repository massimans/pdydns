use crate::config::{AppConfig, RecordTarget};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct PowerDnsUpdateRequest {
    rrsets: Vec<RrsetChange>,
}

#[derive(Debug, Serialize)]
struct RrsetChange {
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    ttl: u32,
    changetype: String,
    records: Vec<RecordContent>,
}

#[derive(Debug, Serialize)]
struct RecordContent {
    content: String,
    disabled: bool,
}

pub async fn update_a_record(
    client: &Client,
    config: &AppConfig,
    record: &RecordTarget,
    ip: &str,
) -> Result<()> {
    let url = format!(
        "{}/api/v1/servers/{}/zones/{}",
        config.powerdns.api_url.trim_end_matches('/'),
        urlencoding::encode(&config.powerdns.server_id),
        urlencoding::encode(&record.normalized_zone()),
    );

    let payload = PowerDnsUpdateRequest {
        rrsets: vec![RrsetChange {
            name: record.normalized_name(),
            record_type: "A".to_string(),
            ttl: record.ttl(config.default_ttl),
            changetype: "REPLACE".to_string(),
            records: vec![RecordContent {
                content: ip.to_string(),
                disabled: false,
            }],
        }],
    };

    let response = client
        .patch(&url)
        .header("X-API-Key", &config.powerdns.api_key)
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("failed to send update request to {url}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(anyhow!(
        "PowerDNS update failed for {} with status {}: {}",
        record.normalized_name(),
        status,
        body
    ))
}
