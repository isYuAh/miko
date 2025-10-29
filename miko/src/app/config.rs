use anyhow::{Error, anyhow};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use toml::Value;
use toml::map::Map;

static CONFIG: OnceLock<Value> = OnceLock::new();

/// 应用层配置（监听地址与端口）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApplicationConfig {
    pub addr: String,
    pub port: u16,
}
impl ApplicationConfig {
    /// 从配置文件加载 ApplicationConfig，失败则返回错误
    pub fn load_() -> Result<Self, Box<dyn std::error::Error>> {
        let base: Value = load_config_section("application")?;
        Ok(base
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
    }
}
impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}
impl ApplicationConfig {
    /// 便于回退时构建默认 toml 值的辅助函数
    pub fn to_hash_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("addr".to_string(), self.addr.clone());
        map.insert("port".to_string(), self.port.to_string());
        map
    }
}

fn merge_toml(base: &mut Value, other: &Value) {
    match (base, other) {
        (Value::Table(base_t), Value::Table(other_t)) => {
            for (k, v) in other_t {
                match base_t.get_mut(k) {
                    Some(bv) => merge_toml(bv, v),
                    None => {
                        base_t.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// 读取并合并配置：config.toml 与 dev/prod 额外文件
pub fn load_and_merge_toml() -> Result<Value, Error> {
    let content = std::fs::read_to_string("./config.toml").inspect_err(|e| {
        tracing::warn!("Failed to read config.toml: {:?}", e);
    })?;
    let mut base: Value = toml::from_str(&content).inspect_err(|e| {
        tracing::warn!("Failed to parse config.toml: {:?}", e);
    })?;
    let env = if cfg!(debug_assertions) {
        "dev"
    } else {
        "prod"
    };
    if let Ok(env_base) = std::fs::read_to_string(format!("./config.{env}.value")) {
        merge_toml(&mut base, &toml::from_str(&env_base)?);
    }
    Ok(base)
}

/// 获取全局配置值（延迟加载并缓存）
pub fn get_config() -> &'static Value {
    CONFIG.get_or_init(|| {
        let val = load_and_merge_toml();
        val.unwrap_or_else(|e| {
            tracing::error!("Failed to load config: {:?}", e);
            let default = ApplicationConfig::default();
            let mut map = Map::new();
            map.insert(
                "application".to_string(),
                Value::from(default.to_hash_map()),
            );
            Value::Table(map)
        })
    })
}

/// 读取指定 section 并反序列化为 T
pub fn load_config_section<T: DeserializeOwned>(section: &str) -> Result<T, Error> {
    let config = get_config();
    let section_value = config
        .get(section)
        .ok_or_else(|| anyhow!("Configuration section '[{}]' not found.", section))?;

    section_value
        .clone()
        .try_into()
        .map_err(|e| anyhow!("Failed to deserialize section '[{}]': {:?}", section, e))
}

/// 从一个 toml::Value 开始，按路径（如 a.b.c）获取子值
pub fn get_toml_value_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = Some(value);
    for key in path.split('.') {
        current = current.and_then(|v| v.get(key));
    }
    current
}
/// 直接从全局配置中按路径取值
pub fn get_config_value(path: &str) -> Option<&Value> {
    get_toml_value_by_path(get_config(), path)
}
