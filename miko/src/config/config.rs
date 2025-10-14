use serde::{Deserialize, Serialize};
use toml::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApplicationConfig {
    pub addr: String,
    pub port: u16,
}
impl ApplicationConfig {
    pub fn load_() -> Result<Self, Box<dyn std::error::Error>> {
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
