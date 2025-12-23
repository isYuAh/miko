use anyhow::{Context, Error};
use config::Config;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;

pub fn load_config_sources() -> Result<Config, Error> {
    let env = env::var("CONFIG_ENV").unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            "dev".to_string()
        } else {
            "prod".to_string()
        }
    });
    Ok(Config::builder()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8080)?
        .add_source(config::File::with_name("./config").required(false))
        .add_source(config::File::with_name(&format!("./config.{}", env)).required(false))
        .add_source(config::Environment::with_prefix("MIKO").separator("__"))
        .build()?)
}

static SETTINGS: OnceLock<Config> = OnceLock::new();

pub fn get_settings() -> &'static Config {
    SETTINGS.get_or_init(|| {
        load_config_sources().expect("Failed to initialize configuration. Check your config files.")
    })
}
pub fn get_settings_value<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let (path, default_val) = match path.rsplit_once(":") {
        Some((p, d)) => (p, Some(d)),
        None => (path, None),
    };
    let settings = get_settings();

    // 尝试从配置源获取
    match settings.get::<T>(path) {
        Ok(v) => Ok(v),
        Err(config_err) => {
            // 如果配置里没找到，且我们有字面量默认值
            if let Some(def_str) = default_val {
                // 尝试解析默认值
                try_parse_default_value(def_str).with_context(|| {
                    format!(
                        "Config key '{}' not found, and failed to parse default literal '{}'",
                        path, def_str
                    )
                })
            } else {
                // 没有默认值，直接抛出原始错误
                Err(config_err.into())
            }
        }
    }
}
fn try_parse_default_value<T: DeserializeOwned>(val: &str) -> Result<T, serde_json::Error> {
    let res = serde_json::from_str::<T>(val);
    res.or_else(|_| serde_json::from_value(serde_json::Value::String(val.to_string())))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}
impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}
impl ServerSettings {
    pub fn from_global_settings() -> Self {
        let settings = get_settings();
        settings.get("server").unwrap_or_else(|_| {
            tracing::warn!("Using default server settings.");
            ServerSettings::default()
        })
    }
}
