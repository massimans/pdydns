use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{env, fs, path::Path, str::FromStr};

pub const POWERDNS_API_URL_ENV: &str = "PDYDNS_POWERDNS_API_URL";
pub const POWERDNS_API_KEY_ENV: &str = "PDYDNS_POWERDNS_API_KEY";
pub const POWERDNS_SERVER_ID_ENV: &str = "PDYDNS_POWERDNS_SERVER_ID";
pub const RECORDS_ENV: &str = "PDYDNS_RECORDS";
pub const INTERVAL_SECONDS_ENV: &str = "PDYDNS_INTERVAL_SECONDS";
pub const DEFAULT_TTL_ENV: &str = "PDYDNS_DEFAULT_TTL";
pub const PUBLIC_IP_URL_ENV: &str = "PDYDNS_PUBLIC_IP_URL";

fn default_interval_seconds() -> u64 {
    300
}

fn default_ttl() -> u32 {
    300
}

fn default_public_ip_url() -> String {
    "https://api4.ipify.org?format=json".to_string()
}

fn default_server_id() -> String {
    "localhost".to_string()
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub powerdns: PowerDnsConfig,
    pub records: Vec<RecordTarget>,
    pub interval_seconds: u64,
    pub default_ttl: u32,
    pub public_ip_url: String,
}

#[derive(Debug, Clone)]
pub struct PowerDnsConfig {
    pub api_url: String,
    pub api_key: String,
    pub server_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AppConfigFile {
    #[serde(default)]
    pub powerdns: PowerDnsConfigFile,
    #[serde(default)]
    pub records: Vec<RecordTarget>,
    pub interval_seconds: Option<u64>,
    pub default_ttl: Option<u32>,
    pub public_ip_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PowerDnsConfigFile {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordTarget {
    pub zone: String,
    pub name: String,
    pub ttl: Option<u32>,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let config_file = if path.exists() {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read config file {}", path.display()))?;
            toml::from_str(&raw)
                .with_context(|| format!("failed to parse config file {}", path.display()))?
        } else {
            AppConfigFile::default()
        };

        let api_url =
            env_or_file_required_string(POWERDNS_API_URL_ENV, config_file.powerdns.api_url)?;
        let api_key =
            env_or_file_required_string(POWERDNS_API_KEY_ENV, config_file.powerdns.api_key)?;
        let server_id = env_or_file_string_with_default(
            POWERDNS_SERVER_ID_ENV,
            config_file.powerdns.server_id,
            default_server_id(),
        )?;
        let interval_seconds = env_or_file_number_with_default(
            INTERVAL_SECONDS_ENV,
            config_file.interval_seconds,
            default_interval_seconds(),
        )?;
        let default_ttl = env_or_file_number_with_default(
            DEFAULT_TTL_ENV,
            config_file.default_ttl,
            default_ttl(),
        )?;
        let public_ip_url = env_or_file_string_with_default(
            PUBLIC_IP_URL_ENV,
            config_file.public_ip_url,
            default_public_ip_url(),
        )?;
        let records = env_records()?.unwrap_or(config_file.records);

        let config = Self {
            powerdns: PowerDnsConfig {
                api_url,
                api_key,
                server_id,
            },
            records,
            interval_seconds,
            default_ttl,
            public_ip_url,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.interval_seconds == 0 {
            return Err(anyhow!("interval_seconds must be greater than zero"));
        }
        if self.records.is_empty() {
            return Err(anyhow!("at least one record must be configured"));
        }
        if self.powerdns.api_url.trim().is_empty() {
            return Err(anyhow!("powerdns.api_url must not be empty"));
        }
        if self.powerdns.api_key.trim().is_empty() {
            return Err(anyhow!("powerdns.api_key must not be empty"));
        }
        for record in &self.records {
            record.validate()?;
        }
        Ok(())
    }
}

fn env_or_file_required_string(env_name: &str, file_value: Option<String>) -> Result<String> {
    match env::var(env_name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => {
            file_value.ok_or_else(|| anyhow!("missing required value: {env_name}"))
        }
        Err(err) => Err(anyhow!("failed to read {env_name}: {err}")),
    }
}

fn env_or_file_string_with_default(
    env_name: &str,
    file_value: Option<String>,
    default_value: String,
) -> Result<String> {
    match env::var(env_name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(file_value.unwrap_or(default_value)),
        Err(err) => Err(anyhow!("failed to read {env_name}: {err}")),
    }
}

fn env_or_file_number_with_default<T>(
    env_name: &str,
    file_value: Option<T>,
    default_value: T,
) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(env_name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|err| anyhow!("failed to parse {env_name}: {err}")),
        Err(env::VarError::NotPresent) => Ok(file_value.unwrap_or(default_value)),
        Err(err) => Err(anyhow!("failed to read {env_name}: {err}")),
    }
}

fn env_records() -> Result<Option<Vec<RecordTarget>>> {
    match env::var(RECORDS_ENV) {
        Ok(value) => {
            let records: Vec<RecordTarget> = serde_json::from_str(&value)
                .with_context(|| format!("failed to parse {RECORDS_ENV} as JSON"))?;
            Ok(Some(records))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(anyhow!("failed to read {RECORDS_ENV}: {err}")),
    }
}

impl RecordTarget {
    pub fn validate(&self) -> Result<()> {
        if self.zone.trim().is_empty() {
            return Err(anyhow!("record zone must not be empty"));
        }
        if self.name.trim().is_empty() {
            return Err(anyhow!("record name must not be empty"));
        }
        Ok(())
    }

    pub fn normalized_zone(&self) -> String {
        normalize_dns_name(&self.zone)
    }

    pub fn normalized_name(&self) -> String {
        normalize_record_name(&self.name, &self.zone)
    }

    pub fn ttl(&self, default_ttl: u32) -> u32 {
        self.ttl.unwrap_or(default_ttl)
    }
}

pub fn normalize_dns_name(value: &str) -> String {
    let value = value.trim();
    if value.ends_with('.') {
        value.to_string()
    } else {
        format!("{value}.")
    }
}

pub fn normalize_record_name(name: &str, zone: &str) -> String {
    let name = name.trim();
    if name.ends_with('.') {
        name.to_string()
    } else if name.contains('.') {
        format!("{name}.")
    } else {
        let zone = normalize_dns_name(zone);
        format!("{name}.{zone}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::Write;
    use std::sync::{LazyLock, Mutex};
    use tempfile::NamedTempFile;

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn normalizes_relative_and_absolute_names() {
        let _guard = ENV_LOCK.lock().unwrap();
        assert_eq!(normalize_dns_name("example.com"), "example.com.");
        assert_eq!(
            normalize_record_name("home", "example.com"),
            "home.example.com."
        );
        assert_eq!(
            normalize_record_name("home.example.com", "example.com"),
            "home.example.com."
        );
        assert_eq!(
            normalize_record_name("home.example.com.", "example.com"),
            "home.example.com."
        );
    }

    #[test]
    fn loads_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        unsafe {
            env::set_var(POWERDNS_API_KEY_ENV, "secret");
        }

        let mut file = NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"
powerdns = {{ api_url = "http://127.0.0.1:8081" }}
interval_seconds = 60
default_ttl = 120

[[records]]
zone = "example.com"
name = "home"
ttl = 600
"#
        )
        .expect("write config");

        let config = AppConfig::load(file.path()).expect("config");
        assert_eq!(config.interval_seconds, 60);
        assert_eq!(config.default_ttl, 120);
        assert_eq!(config.records.len(), 1);
        assert_eq!(config.powerdns.api_key, "secret");
        assert_eq!(config.records[0].normalized_name(), "home.example.com.");
        assert_eq!(config.records[0].ttl(config.default_ttl), 600);
    }

    #[test]
    fn env_overrides_file_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        unsafe {
            env::set_var(POWERDNS_API_URL_ENV, "http://override:8081");
            env::set_var(POWERDNS_API_KEY_ENV, "env-secret");
            env::set_var(POWERDNS_SERVER_ID_ENV, "env-server");
            env::set_var(INTERVAL_SECONDS_ENV, "30");
            env::set_var(DEFAULT_TTL_ENV, "90");
            env::set_var(PUBLIC_IP_URL_ENV, "https://example.com/ip");
            env::set_var(
                RECORDS_ENV,
                r#"[{"zone":"env.com","name":"home","ttl":42}]"#,
            );
        }

        let mut file = NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"
powerdns = {{ api_url = "http://127.0.0.1:8081", api_key = "file-secret", server_id = "file-server" }}
interval_seconds = 60
default_ttl = 120
public_ip_url = "https://file.example.com/ip"

[[records]]
zone = "file.com"
name = "home"
ttl = 600
"#
        )
        .expect("write config");

        let config = AppConfig::load(file.path()).expect("config");
        assert_eq!(config.powerdns.api_url, "http://override:8081");
        assert_eq!(config.powerdns.api_key, "env-secret");
        assert_eq!(config.powerdns.server_id, "env-server");
        assert_eq!(config.interval_seconds, 30);
        assert_eq!(config.default_ttl, 90);
        assert_eq!(config.public_ip_url, "https://example.com/ip");
        assert_eq!(config.records.len(), 1);
        assert_eq!(config.records[0].zone, "env.com");
        assert_eq!(config.records[0].name, "home");
        assert_eq!(config.records[0].ttl, Some(42));
    }

    #[test]
    fn loads_from_env_without_config_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        unsafe {
            env::set_var(POWERDNS_API_URL_ENV, "http://env-only:8081");
            env::set_var(POWERDNS_API_KEY_ENV, "env-only-secret");
            env::set_var(RECORDS_ENV, r#"[{"zone":"env.com","name":"home"}]"#);
        }

        let config = AppConfig::load("/definitely/not/present/config.toml").expect("config");
        assert_eq!(config.powerdns.api_url, "http://env-only:8081");
        assert_eq!(config.powerdns.api_key, "env-only-secret");
        assert_eq!(config.records.len(), 1);
        assert_eq!(config.records[0].zone, "env.com");
    }

    fn clear_env() {
        unsafe {
            env::remove_var(POWERDNS_API_URL_ENV);
            env::remove_var(POWERDNS_API_KEY_ENV);
            env::remove_var(POWERDNS_SERVER_ID_ENV);
            env::remove_var(RECORDS_ENV);
            env::remove_var(INTERVAL_SECONDS_ENV);
            env::remove_var(DEFAULT_TTL_ENV);
            env::remove_var(PUBLIC_IP_URL_ENV);
        }
    }
}
