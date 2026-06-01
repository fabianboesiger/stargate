use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    pub default_server: Option<String>,
    pub spacetimedb_token: Option<String>,
    #[serde(default)]
    pub server_configs: Vec<ServerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub nickname: String,
    pub host: String,
    pub protocol: String,
}

impl ServerConfig {
    pub fn url(&self) -> String {
        format!("{}://{}", self.protocol, self.host)
    }
}

impl CliConfig {
    /// Attempt to find and load the SpacetimeDB CLI config from the standard XDG location.
    /// On macOS/Linux: ~/.config/spacetime/cli.toml
    pub fn load() -> Option<Self> {
        let path = Self::config_path()?;
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    fn config_path() -> Option<PathBuf> {
        // Unix (Linux & macOS): $XDG_CONFIG_HOME/spacetime/cli.toml
        // Defaults to ~/.config/spacetime/cli.toml
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config).join("spacetime").join("cli.toml");
            if path.exists() {
                return Some(path);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let xdg_path = home.join(".config").join("spacetime").join("cli.toml");
            if xdg_path.exists() {
                return Some(xdg_path);
            }
        }
        // Windows: %LocalAppData%\SpacetimeDB\config\cli.toml
        #[cfg(windows)]
        if let Some(local_data) = dirs::data_local_dir() {
            let path = local_data.join("SpacetimeDB").join("config").join("cli.toml");
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    pub fn default_server_config(&self) -> Option<&ServerConfig> {
        let default_name = self.default_server.as_deref()?;
        self.server_configs
            .iter()
            .find(|s| s.nickname == default_name || s.host == default_name)
    }
}
