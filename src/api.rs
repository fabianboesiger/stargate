use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    pub base_url: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseEntry {
    pub identity: String,
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct IdentitiesResponse {
    identities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NamesResponse {
    names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DatabaseInfo {
    #[serde(default)]
    pub database_identity: String,
    #[serde(default)]
    pub owner_identity: String,
    #[serde(default)]
    pub host_type: String,
    #[serde(default)]
    pub initial_program: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SchemaResponse {
    #[serde(default)]
    pub tables: Vec<TableSchema>,
    #[serde(default)]
    pub reducers: Vec<ReducerSchema>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TableSchema {
    pub name: String,
    #[serde(default)]
    pub primary_key: Vec<u32>,
    #[serde(default)]
    pub indexes: Vec<serde_json::Value>,
    #[serde(default)]
    pub constraints: Vec<serde_json::Value>,
    #[serde(default)]
    pub table_type: serde_json::Value,
    #[serde(default)]
    pub table_access: serde_json::Value,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReducerSchema {
    pub name: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ReducerSchema {
    pub fn params(&self) -> Vec<ReducerParam> {
        log::info!("ReducerSchema '{}' extra keys: {:?}", self.name, self.extra.keys().collect::<Vec<_>>());
        for key in &["params", "args", "parameters", "schema"] {
            if let Some(val) = self.extra.get(*key) {
                log::info!("  Found key '{}': {}", key, &format!("{val}")[..format!("{val}").len().min(500)]);
                if let Some(params) = Self::parse_params(val) {
                    log::info!("  Parsed {} params", params.len());
                    return params;
                }
                log::info!("  Failed to parse params from key '{}'", key);
            }
        }
        Vec::new()
    }

    fn parse_params(val: &serde_json::Value) -> Option<Vec<ReducerParam>> {
        // Try direct Vec<ReducerParam>
        if let Ok(params) = serde_json::from_value::<Vec<ReducerParam>>(val.clone())
            && !params.is_empty()
        {
            return Some(params);
        }

        // Try ProductType format: {"elements": [...]}
        let elements = if let Some(obj) = val.as_object() {
            obj.get("elements").and_then(|e| e.as_array())
        } else {
            val.as_array()
        };

        if let Some(elements) = elements {
            let params: Vec<ReducerParam> = elements
                .iter()
                .enumerate()
                .map(|(i, elem)| {
                    let name = Self::extract_name(elem)
                        .unwrap_or_else(|| format!("arg{i}"));
                    let ty = elem.get("algebraic_type")
                        .or_else(|| elem.get("type"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    ReducerParam { name, ty }
                })
                .collect();
            if !params.is_empty() {
                return Some(params);
            }
        }

        None
    }

    /// Extract the name from a ProductTypeElement, handling SATS Option encoding
    fn extract_name(elem: &serde_json::Value) -> Option<String> {
        let name_val = elem.get("name")?;
        // Plain string
        if let Some(s) = name_val.as_str() {
            return Some(s.to_string());
        }
        // SATS Option encoding: {"some": "name"}
        if let Some(obj) = name_val.as_object()
            && let Some(inner) = obj.get("some")
            && let Some(s) = inner.as_str()
        {
            return Some(s.to_string());
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReducerParam {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type", alias = "algebraic_type")]
    pub ty: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub ts: u64,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub function: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogStreamMessage {
    Entry(LogEntry),
    Error(String),
    Disconnected,
}

#[derive(Debug, Clone, Deserialize)]
struct SqlResultSet {
    schema: SqlSchema,
    #[serde(default)]
    rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SqlSchema {
    elements: Vec<SqlSchemaElement>,
}

#[derive(Debug, Clone, Deserialize)]
struct SqlSchemaElement {
    #[serde(default)]
    name: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[allow(dead_code)]
impl ApiClient {
    pub fn new(base_url: &str, token: Option<&str>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.map(|t| t.to_string()),
        }
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    /// Get database info (identity, owner, host_type, initial_program).
    pub async fn get_database_info(&self, db_identity: &str) -> Result<DatabaseInfo, String> {
        let url = format!("{}/v1/database/{}", self.base_url, db_identity);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    /// List databases owned by the given identity, including their names.
    pub async fn list_databases(&self, identity: &str) -> Result<Vec<DatabaseEntry>, String> {
        let url = format!("{}/v1/identity/{}/databases", self.base_url, identity);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        let ids: IdentitiesResponse = resp.json().await.map_err(|e| e.to_string())?;

        let mut databases = Vec::new();
        for db_identity in ids.identities {
            let names = self.get_database_names(&db_identity).await.unwrap_or_default();
            databases.push(DatabaseEntry {
                identity: db_identity,
                names,
            });
        }
        Ok(databases)
    }

    /// Get the registered names for a database.
    pub async fn get_database_names(&self, db_identity: &str) -> Result<Vec<String>, String> {
        let url = format!("{}/v1/database/{}/names", self.base_url, db_identity);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let names: NamesResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(names.names)
    }

    /// Get the schema (tables, reducers) for a database.
    pub async fn get_schema(&self, db_identity: &str) -> Result<SchemaResponse, String> {
        let url = format!("{}/v1/database/{}/schema?version=9", self.base_url, db_identity);
        log::info!("Fetching schema from: {url}");
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        let body = resp.text().await.map_err(|e| e.to_string())?;
        log::info!("Schema response (first 1000 chars): {}", &body[..body.len().min(1000)]);
        serde_json::from_str(&body).map_err(|e| {
            log::error!("Schema parse error: {e}");
            log::error!("Schema response (first 2000 chars): {}", &body[..body.len().min(2000)]);
            format!("error decoding response body: {e}")
        })
    }

    /// Get recent logs for a database.
    pub async fn get_logs(&self, db_identity: &str, num_lines: u32) -> Result<Vec<LogEntry>, String> {
        let url = format!("{}/v1/database/{}/logs?num_lines={}", self.base_url, db_identity, num_lines);
        log::info!("Fetching logs from: {url}");
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        // Response is newline-delimited JSON
        let body = resp.text().await.map_err(|e| e.to_string())?;
        let entries: Vec<LogEntry> = body
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(entries)
    }

    /// Stream logs from a database using follow=true.
    /// Returns initial logs plus a channel that receives new log entries as they arrive.
    pub fn subscribe_logs(
        &self,
        db_identity: &str,
        num_lines: u32,
    ) -> tokio::sync::mpsc::UnboundedReceiver<LogStreamMessage> {
        use futures::StreamExt;

        let url = format!(
            "{}/v1/database/{}/logs?num_lines={}&follow=true",
            self.base_url, db_identity, num_lines
        );
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(LogStreamMessage::Error(e.to_string()));
                    return;
                }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                let _ = tx.send(LogStreamMessage::Error(format!("HTTP {status}: {body}")));
                return;
            }

            let mut stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(LogStreamMessage::Error(e.to_string()));
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Ok(entry) = serde_json::from_str::<LogEntry>(&line)
                        && tx.send(LogStreamMessage::Entry(entry)).is_err()
                    {
                        return; // receiver dropped
                    }
                }
            }

            let _ = tx.send(LogStreamMessage::Disconnected);
        });

        rx
    }

    /// Execute a SQL query against a database.
    pub async fn execute_sql(&self, db_identity: &str, query: &str) -> Result<SqlResult, String> {
        let url = format!("{}/v1/database/{}/sql", self.base_url, db_identity);
        let mut req = self.client.post(&url)
            .header("Content-Type", "text/plain")
            .body(query.to_string());
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        let results: Vec<SqlResultSet> = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(result_set) = results.into_iter().next() {
            let columns: Vec<String> = result_set
                .schema
                .elements
                .iter()
                .filter_map(|e| e.name.get("some").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect();
            Ok(SqlResult {
                columns,
                rows: result_set.rows,
            })
        } else {
            Ok(SqlResult {
                columns: Vec::new(),
                rows: Vec::new(),
            })
        }
    }

    /// Call a reducer on a database with the given JSON arguments.
    pub async fn call_reducer(&self, db_identity: &str, reducer_name: &str, args: &[serde_json::Value]) -> Result<String, String> {
        let url = format!("{}/v1/database/{}/call/{}", self.base_url, db_identity, reducer_name);
        log::info!("Calling reducer: {reducer_name} on {db_identity}");
        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string()));
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }
        let body = resp.text().await.unwrap_or_default();
        Ok(body)
    }
}

// --- OAuth Login Flow ---

const DEFAULT_AUTH_HOST: &str = "https://spacetimedb.com";

#[derive(Deserialize)]
struct AuthTokenData {
    token: String,
}

#[derive(Deserialize)]
struct AuthTokenResponse {
    success: bool,
    data: AuthTokenData,
}

#[derive(Deserialize)]
struct AuthSessionData {
    approved: bool,
    #[serde(rename = "sessionToken")]
    session_token: Option<String>,
}

#[derive(Deserialize)]
struct AuthSessionResponse {
    success: bool,
    error: Option<String>,
    data: Option<AuthSessionData>,
}

#[derive(Deserialize)]
struct SpacetimeDBTokenData {
    token: String,
}

#[derive(Deserialize)]
struct SpacetimeDBTokenResponse {
    success: bool,
    error: Option<String>,
    data: Option<SpacetimeDBTokenData>,
}

/// Request a login token and return the browser URL + the request token for polling.
pub async fn oauth_request_login(auth_host: Option<&str>) -> Result<(String, String), String> {
    let host = auth_host.unwrap_or(DEFAULT_AUTH_HOST);
    let client = reqwest::Client::new();

    let url = format!("{}/api/auth/cli/login/request-token", host.trim_end_matches('/'));
    log::info!("OAuth: requesting login token from {url}");

    let resp: AuthTokenResponse = client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request login token: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse login token response: {e}"))?;

    if !resp.success {
        return Err("Auth server returned failure for token request".to_string());
    }

    let request_token = resp.data.token;
    let browser_url = format!(
        "{}/login/cli?token={}",
        host.trim_end_matches('/'),
        request_token
    );

    Ok((browser_url, request_token))
}

/// Poll the auth server until the user approves the login.
/// Returns the session token on success.
pub async fn oauth_poll_approval(auth_host: Option<&str>, request_token: &str) -> Result<String, String> {
    let host = auth_host.unwrap_or(DEFAULT_AUTH_HOST);
    let client = reqwest::Client::new();
    let status_url = format!(
        "{}/api/auth/cli/status?token={}",
        host.trim_end_matches('/'),
        request_token
    );

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let resp: AuthSessionResponse = client
            .get(&status_url)
            .send()
            .await
            .map_err(|e| format!("Failed to poll login status: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse login status response: {e}"))?;

        if !resp.success {
            let err = resp.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(format!("Auth server error: {err}"));
        }

        if let Some(data) = resp.data
            && data.approved
        {
            if let Some(session_token) = data.session_token {
                log::info!("OAuth: login approved");
                return Ok(session_token);
            }
            return Err("Login approved but session token is missing".to_string());
        }
        // Not yet approved, continue polling
    }
}

/// Exchange a web session token for a SpacetimeDB token.
pub async fn oauth_exchange_token(auth_host: Option<&str>, session_token: &str) -> Result<String, String> {
    let host = auth_host.unwrap_or(DEFAULT_AUTH_HOST);
    let client = reqwest::Client::new();

    let url = format!("{}/api/spacetimedb-token", host.trim_end_matches('/'));
    log::info!("OAuth: exchanging session token for SpacetimeDB token");

    let resp: SpacetimeDBTokenResponse = client
        .post(&url)
        .header("Authorization", format!("Bearer {session_token}"))
        .send()
        .await
        .map_err(|e| format!("Failed to exchange token: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse token exchange response: {e}"))?;

    if !resp.success {
        let err = resp.error.unwrap_or_else(|| "Unknown error".to_string());
        return Err(format!("Token exchange failed: {err}"));
    }

    resp.data
        .map(|d| d.token)
        .ok_or_else(|| "Token exchange response missing data".to_string())
}
