use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::net::Ipv4Addr;

#[derive(Debug, Deserialize)]
struct IpResponse {
    ip: String,
}

pub async fn fetch_public_ip(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to query public ip service {url}"))?
        .error_for_status()
        .with_context(|| format!("public ip service returned an error for {url}"))?;

    let body = response
        .text()
        .await
        .with_context(|| format!("failed to read public ip response from {url}"))?;

    let ip = match serde_json::from_str::<IpResponse>(&body) {
        Ok(parsed) => parsed.ip,
        Err(_) => body.trim().to_string(),
    };

    let parsed_ip: Ipv4Addr = ip
        .parse()
        .map_err(|_| anyhow!("public ip service returned a non-IPv4 address: {ip}"))?;
    Ok(parsed_ip.to_string())
}
