use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config<'a> {
    pub homeserver_url: Cow<'a, str>,
    pub mxid: Cow<'a, str>,
    pub password: Cow<'a, str>,
}

impl Config<'_> {
    pub fn load<P: AsRef<std::path::Path> + std::fmt::Debug>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
