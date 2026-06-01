use crate::api::ApiClient;
use crate::config::{CliConfig, ServerConfig};

/// Represents the current connection state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    pub cli_config: Option<CliConfig>,
    pub connected: bool,
    pub server_url: String,
    pub token: String,
    pub identity: String,
    pub api: Option<ApiClient>,
    pub readonly: bool,
}

impl AppState {
    pub fn new() -> Self {
        let cli_config = CliConfig::load();
        Self {
            cli_config,
            connected: false,
            server_url: String::new(),
            token: String::new(),
            identity: String::new(),
            api: None,
            readonly: true,
        }
    }

    pub fn available_servers(&self) -> Vec<&ServerConfig> {
        self.cli_config
            .as_ref()
            .map(|c| {
                let default_name = c.default_server.as_deref().unwrap_or("");
                let mut servers: Vec<&ServerConfig> = c.server_configs.iter().collect();
                servers.sort_by_key(|s| {
                    if s.nickname == default_name || s.host == default_name {
                        0
                    } else {
                        1
                    }
                });
                servers
            })
            .unwrap_or_default()
    }

    pub fn default_server_url(&self) -> Option<String> {
        self.cli_config
            .as_ref()
            .and_then(|c| c.default_server_config())
            .map(|s| s.url())
    }

    pub fn cli_token(&self) -> Option<&str> {
        self.cli_config
            .as_ref()
            .and_then(|c| c.spacetimedb_token.as_deref())
    }

    pub fn connect(&mut self, server_url: &str, token: &str, identity: &str) {
        self.server_url = server_url.to_string();
        self.token = token.to_string();
        self.identity = identity.to_string();
        self.api = Some(ApiClient::new(server_url, Some(token)));
        self.connected = true;
    }
}
