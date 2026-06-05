//! Runtime OpenAPI 3.1 spec generation for the connected SpacetimeDB database.
//!
//! Stargate is a *client* of SpacetimeDB's HTTP/WebSocket API, so there are no
//! hand-written routes to annotate. Instead, [`build_spec`] reconstructs the full
//! API surface of the *connected* database — the authentication endpoints, the
//! static REST endpoints, the websocket subscribe endpoint, and one concrete
//! `POST .../call/{reducer}` entry per reducer discovered in the live schema.
//!
//! The result is a plain [`serde_json::Value`] OpenAPI document that is both
//! rendered (via RapiDoc) and downloadable as `openapi.json`.

use serde_json::{json, Map, Value};

use crate::api::SchemaResponse;

/// Build an OpenAPI 3.1 document for the given database.
///
/// * `base_url` — the SpacetimeDB host the database lives on (e.g. `https://spacetimedb.com`).
/// * `auth_host` — the authentication host (usually the same as `base_url`).
/// * `db_identity` — the concrete database identity, baked into every path so the
///   spec is immediately usable.
/// * `schema` — the live schema, used to expand per-reducer endpoints.
pub fn build_spec(
    base_url: &str,
    auth_host: &str,
    db_identity: &str,
    schema: &SchemaResponse,
) -> Value {
    let base_url = base_url.trim_end_matches('/');
    let auth_host = auth_host.trim_end_matches('/');

    let mut paths = Map::new();

    // --- Authentication (CLI OAuth flow) ---
    paths.insert(
        format!("{auth_host}/api/auth/cli/login/request-token"),
        json!({
            "post": {
                "tags": ["Authentication"],
                "summary": "Request a CLI login token",
                "description": "Starts the CLI OAuth flow. Returns a request token; the user then approves the login in a browser at `/login/cli?token={token}`.",
                "security": [],
                "responses": ok_json_response("The request token.", "#/components/schemas/AuthTokenResponse"),
            }
        }),
    );
    paths.insert(
        format!("{auth_host}/api/auth/cli/status"),
        json!({
            "get": {
                "tags": ["Authentication"],
                "summary": "Poll CLI login approval status",
                "description": "Polls until the user approves the login request in the browser. Returns a web session token once approved.",
                "security": [],
                "parameters": [{
                    "name": "token",
                    "in": "query",
                    "required": true,
                    "description": "The request token returned by `request-token`.",
                    "schema": { "type": "string" }
                }],
                "responses": ok_json_response("The current approval status.", "#/components/schemas/AuthSessionResponse"),
            }
        }),
    );
    paths.insert(
        format!("{auth_host}/api/spacetimedb-token"),
        json!({
            "post": {
                "tags": ["Authentication"],
                "summary": "Exchange a web session token for a SpacetimeDB token",
                "description": "Exchanges the approved web session token (sent as a Bearer token) for a long-lived SpacetimeDB identity token used to authenticate all database requests.",
                "security": [{ "bearerAuth": [] }],
                "responses": ok_json_response("The SpacetimeDB token.", "#/components/schemas/SpacetimeDBTokenResponse"),
            }
        }),
    );

    // --- Identity ---
    paths.insert(
        "/v1/identity/{identity}/databases".to_string(),
        json!({
            "get": {
                "tags": ["Database"],
                "summary": "List databases owned by an identity",
                "parameters": [identity_param()],
                "responses": ok_json_response("The owned database identities.", "#/components/schemas/IdentitiesResponse"),
            }
        }),
    );

    // --- Database (REST) ---
    paths.insert(
        format!("/v1/database/{db_identity}"),
        json!({
            "get": {
                "tags": ["Database"],
                "summary": "Get database info",
                "description": "Returns identity, owner, host type and the initial program hash.",
                "responses": ok_json_response("Database information.", "#/components/schemas/DatabaseInfo"),
            }
        }),
    );
    paths.insert(
        format!("/v1/database/{db_identity}/names"),
        json!({
            "get": {
                "tags": ["Database"],
                "summary": "Get registered DNS names for the database",
                "responses": ok_json_response("The registered names.", "#/components/schemas/NamesResponse"),
            }
        }),
    );
    paths.insert(
        format!("/v1/database/{db_identity}/schema"),
        json!({
            "get": {
                "tags": ["Database"],
                "summary": "Get the database schema",
                "description": "Returns the full schema: tables, reducers, indexes and constraints.",
                "parameters": [{
                    "name": "version",
                    "in": "query",
                    "required": true,
                    "description": "Schema format version (Stargate uses `9`).",
                    "schema": { "type": "integer", "default": 9 }
                }],
                "responses": ok_json_response("The database schema.", "#/components/schemas/SchemaResponse"),
            }
        }),
    );

    // --- Logs ---
    paths.insert(
        format!("/v1/database/{db_identity}/logs"),
        json!({
            "get": {
                "tags": ["Logs"],
                "summary": "Fetch (or stream) database logs",
                "description": "Returns the most recent log lines as newline-delimited JSON. With `follow=true` the connection stays open and streams new entries as they arrive (long-polling).",
                "parameters": [
                    {
                        "name": "num_lines",
                        "in": "query",
                        "required": false,
                        "description": "Number of recent log lines to return.",
                        "schema": { "type": "integer", "default": 100 }
                    },
                    {
                        "name": "follow",
                        "in": "query",
                        "required": false,
                        "description": "Stream new log entries as they arrive.",
                        "schema": { "type": "boolean", "default": false }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Newline-delimited JSON log entries.",
                        "content": {
                            "application/x-ndjson": {
                                "schema": { "$ref": "#/components/schemas/LogEntry" }
                            }
                        }
                    }
                },
            }
        }),
    );

    // --- SQL ---
    paths.insert(
        format!("/v1/database/{db_identity}/sql"),
        json!({
            "post": {
                "tags": ["SQL"],
                "summary": "Execute a SQL query",
                "description": "Runs a SQL statement against the database. The request body is the raw SQL string.",
                "requestBody": {
                    "required": true,
                    "content": {
                        "text/plain": {
                            "schema": { "type": "string", "example": "SELECT * FROM my_table LIMIT 10" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "An array of result sets (schema + rows).",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/SqlResultSet" }
                                }
                            }
                        }
                    }
                },
            }
        }),
    );

    // --- Reducers (one concrete path per reducer) ---
    paths.insert(
        format!("/v1/database/{db_identity}/call/{{reducer}}"),
        json!({
            "post": {
                "tags": ["Reducers"],
                "summary": "Call a reducer (generic)",
                "description": "Invokes any reducer by name. The request body is a JSON array of positional arguments. See the individual reducer endpoints below for typed argument schemas.",
                "parameters": [{
                    "name": "reducer",
                    "in": "path",
                    "required": true,
                    "description": "The reducer name.",
                    "schema": { "type": "string" }
                }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "type": "array", "items": {} }
                        }
                    }
                },
                "responses": { "200": { "description": "The reducer's return value (if any)." } },
            }
        }),
    );

    for reducer in &schema.reducers {
        let params = reducer.params();
        let mut prefix_items: Vec<Value> = Vec::with_capacity(params.len());
        let mut arg_desc: Vec<String> = Vec::with_capacity(params.len());
        let mut example: Vec<Value> = Vec::with_capacity(params.len());
        for (i, p) in params.iter().enumerate() {
            let name = if p.name.is_empty() { format!("arg{i}") } else { p.name.clone() };
            let mut schema = sats_to_schema(&p.ty);
            let ex = example_value(&p.ty, &name);
            if let Some(obj) = schema.as_object_mut() {
                obj.entry("title").or_insert(json!(name));
                obj.entry("example").or_insert(ex.clone());
            }
            arg_desc.push(format!("`{name}`: {}", type_label(&p.ty)));
            prefix_items.push(schema);
            example.push(ex);
        }

        let n = prefix_items.len();
        let body_schema = json!({
            "type": "array",
            "prefixItems": prefix_items,
            "minItems": n,
            "maxItems": n,
            // A top-level example so RapiDoc's "Fill Example" produces a populated
            // array — it does not synthesize examples from `prefixItems` tuples.
            "example": example.clone(),
            "description": if arg_desc.is_empty() {
                "No arguments.".to_string()
            } else {
                format!("Positional arguments — {}", arg_desc.join(", "))
            }
        });

        paths.insert(
            format!("/v1/database/{db_identity}/call/{}", reducer.name),
            json!({
                "post": {
                    "tags": ["Reducers"],
                    "summary": format!("Call `{}`", reducer.name),
                    "description": format!(
                        "Invokes the `{}` reducer. The request body is a JSON array of {} positional argument(s).",
                        reducer.name, n
                    ),
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": body_schema,
                                "example": Value::Array(example),
                            }
                        }
                    },
                    "responses": { "200": { "description": "The reducer executed." } },
                }
            }),
        );
    }

    // --- WebSocket (subscribe) ---
    paths.insert(
        format!("/v1/database/{db_identity}/subscribe"),
        json!({
            "get": {
                "tags": ["WebSocket"],
                "summary": "Subscribe to real-time table updates (WebSocket)",
                "description": concat!(
                    "Upgrades the connection to a WebSocket for real-time subscriptions. ",
                    "OpenAPI cannot natively describe WebSocket messaging, so this is documented as the upgrade handshake.\n\n",
                    "**Handshake headers**\n",
                    "- `Connection: Upgrade`\n",
                    "- `Upgrade: websocket`\n",
                    "- `Sec-WebSocket-Protocol: v1.json.spacetimedb`\n",
                    "- `Authorization: Bearer <token>`\n\n",
                    "**Client → server messages**: see `ClientMessage` (e.g. `SubscribeSingle`).\n\n",
                    "**Server → client messages**: `InitialSubscription`, `SubscribeApplied`, ",
                    "`TransactionUpdate`, `TransactionUpdateLight`, `SubscriptionError`, `IdentityToken`."
                ),
                "parameters": [
                    {
                        "name": "Connection",
                        "in": "header",
                        "required": true,
                        "schema": { "type": "string", "default": "Upgrade" }
                    },
                    {
                        "name": "Upgrade",
                        "in": "header",
                        "required": true,
                        "schema": { "type": "string", "default": "websocket" }
                    },
                    {
                        "name": "Sec-WebSocket-Protocol",
                        "in": "header",
                        "required": true,
                        "schema": { "type": "string", "default": "v1.json.spacetimedb" }
                    }
                ],
                "requestBody": {
                    "description": "WebSocket client message sent after the handshake.",
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ClientMessage" } } }
                },
                "responses": {
                    "101": { "description": "Switching Protocols — the WebSocket is established." }
                },
            }
        }),
    );

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "SpacetimeDB API",
            "version": "v1",
            "description": format!(
                "Auto-generated OpenAPI specification for the SpacetimeDB database `{db_identity}`, \
                 produced by Stargate from the live schema. Covers authentication, REST endpoints, \
                 the WebSocket subscribe endpoint, and every reducer of this database."
            )
        },
        "servers": [
            { "url": base_url, "description": "SpacetimeDB database host" },
            { "url": auth_host, "description": "SpacetimeDB authentication host" }
        ],
        "tags": [
            { "name": "Authentication", "description": "CLI OAuth login flow (on the authentication host)." },
            { "name": "Database", "description": "Database metadata and schema." },
            { "name": "SQL", "description": "Ad-hoc SQL queries." },
            { "name": "Reducers", "description": "Reducer invocation endpoints." },
            { "name": "Logs", "description": "Database log retrieval and streaming." },
            { "name": "WebSocket", "description": "Real-time subscription endpoint." }
        ],
        "security": [{ "bearerAuth": [] }],
        "paths": Value::Object(paths),
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "description": "SpacetimeDB identity token, sent as `Authorization: Bearer <token>`."
                }
            },
            "schemas": component_schemas(),
        }
    })
}

/// A standard `200 application/json` response referencing a component schema.
fn ok_json_response(description: &str, schema_ref: &str) -> Value {
    json!({
        "200": {
            "description": description,
            "content": { "application/json": { "schema": { "$ref": schema_ref } } }
        }
    })
}

fn identity_param() -> Value {
    json!({
        "name": "identity",
        "in": "path",
        "required": true,
        "description": "The owner identity.",
        "schema": { "type": "string" }
    })
}

/// Best-effort mapping of a SATS algebraic type (as it appears in the schema JSON)
/// to a JSON Schema fragment. Falls back to an unconstrained schema for composite
/// or unrecognised types.
fn sats_to_schema(ty: &Value) -> Value {
    if let Some(tag) = scalar_tag(ty) {
        return match tag {
            "Bool" => json!({ "type": "boolean" }),
            "F32" | "F64" => json!({ "type": "number" }),
            "I8" | "I16" | "I32" | "U8" | "U16" | "U32" => json!({ "type": "integer" }),
            // 64/128-bit integers are commonly transmitted as strings to avoid precision loss.
            "I64" | "U64" | "I128" | "U128" | "I256" | "U256" => {
                json!({ "type": ["integer", "string"] })
            }
            "String" => json!({ "type": "string" }),
            _ => json!({}),
        };
    }
    // Array types: {"Array": <element type>}
    if let Some(obj) = ty.as_object()
        && let Some(elem) = obj.get("Array")
    {
        return json!({ "type": "array", "items": sats_to_schema(elem) });
    }
    // Composite / referenced types — leave unconstrained but keep a hint.
    json!({ "description": format!("SATS type: {}", type_label(ty)) })
}

/// A placeholder example value for a SATS type, used to pre-fill request bodies.
/// `name` is the argument name, used to make string placeholders readable.
fn example_value(ty: &Value, name: &str) -> Value {
    if let Some(tag) = scalar_tag(ty) {
        return match tag {
            "Bool" => json!(false),
            "F32" | "F64" => json!(0),
            "I8" | "I16" | "I32" | "U8" | "U16" | "U32" => json!(0),
            // 64/128-bit integers travel as strings to avoid precision loss.
            "I64" | "U64" | "I128" | "U128" | "I256" | "U256" => json!("0"),
            "String" => json!(format!("<{name}>")),
            _ => Value::Null,
        };
    }
    if let Some(obj) = ty.as_object() {
        if let Some(elem) = obj.get("Array") {
            return json!([example_value(elem, name)]);
        }
        if obj.contains_key("Product") || obj.contains_key("Sum") {
            return json!({});
        }
    }
    Value::Null
}

/// If `ty` is a single-key object whose key is a known scalar tag, return that tag.
fn scalar_tag(ty: &Value) -> Option<&str> {
    let obj = ty.as_object()?;
    if obj.len() != 1 {
        return None;
    }
    let key = obj.keys().next()?.as_str();
    const SCALARS: &[&str] = &[
        "Bool", "I8", "I16", "I32", "I64", "I128", "I256", "U8", "U16", "U32", "U64", "U128",
        "U256", "F32", "F64", "String",
    ];
    SCALARS.contains(&key).then_some(key)
}

/// A short human-readable label for a SATS type, used in descriptions.
fn type_label(ty: &Value) -> String {
    if let Some(tag) = scalar_tag(ty) {
        return tag.to_string();
    }
    if let Some(obj) = ty.as_object() {
        if obj.contains_key("Array") {
            return "Array".to_string();
        }
        if obj.contains_key("Product") {
            return "Product".to_string();
        }
        if obj.contains_key("Sum") {
            return "Sum".to_string();
        }
        if obj.contains_key("Ref") {
            return "Ref".to_string();
        }
        if let Some(key) = obj.keys().next() {
            return key.clone();
        }
    }
    "any".to_string()
}

/// The reusable component schemas mirrored from the structs in [`crate::api`].
fn component_schemas() -> Value {
    json!({
        "DatabaseInfo": {
            "type": "object",
            "properties": {
                "database_identity": { "type": "string" },
                "owner_identity": { "type": "string" },
                "host_type": { "type": "string" },
                "initial_program": { "type": "string" }
            }
        },
        "IdentitiesResponse": {
            "type": "object",
            "properties": {
                "identities": { "type": "array", "items": { "type": "string" } }
            }
        },
        "NamesResponse": {
            "type": "object",
            "properties": {
                "names": { "type": "array", "items": { "type": "string" } }
            }
        },
        "SchemaResponse": {
            "type": "object",
            "properties": {
                "tables": { "type": "array", "items": { "$ref": "#/components/schemas/TableSchema" } },
                "reducers": { "type": "array", "items": { "$ref": "#/components/schemas/ReducerSchema" } }
            }
        },
        "TableSchema": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "primary_key": { "type": "array", "items": { "type": "integer" } },
                "indexes": { "type": "array", "items": {} },
                "constraints": { "type": "array", "items": {} },
                "table_type": {},
                "table_access": {}
            },
            "required": ["name"]
        },
        "ReducerSchema": {
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"],
            "description": "A reducer definition; argument types are described per-endpoint above."
        },
        "LogEntry": {
            "type": "object",
            "properties": {
                "level": { "type": "string" },
                "ts": { "type": "integer" },
                "target": { "type": "string" },
                "function": { "type": "string" },
                "message": { "type": "string" }
            }
        },
        "SqlResultSet": {
            "type": "object",
            "properties": {
                "schema": { "type": "object", "description": "Column schema (ProductType `elements`)." },
                "rows": { "type": "array", "items": { "type": "array", "items": {} } }
            }
        },
        "AuthTokenResponse": {
            "type": "object",
            "properties": {
                "success": { "type": "boolean" },
                "data": { "type": "object", "properties": { "token": { "type": "string" } } }
            }
        },
        "AuthSessionResponse": {
            "type": "object",
            "properties": {
                "success": { "type": "boolean" },
                "error": { "type": ["string", "null"] },
                "data": {
                    "type": "object",
                    "properties": {
                        "approved": { "type": "boolean" },
                        "session_token": { "type": ["string", "null"] }
                    }
                }
            }
        },
        "SpacetimeDBTokenResponse": {
            "type": "object",
            "properties": {
                "success": { "type": "boolean" },
                "error": { "type": ["string", "null"] },
                "data": { "type": "object", "properties": { "token": { "type": "string" } } }
            }
        },
        "ClientMessage": {
            "type": "object",
            "description": "Externally-tagged WebSocket client message.",
            "properties": {
                "SubscribeSingle": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "example": "SELECT * FROM my_table" },
                        "request_id": { "type": "integer" },
                        "query_id": {
                            "type": "array",
                            "prefixItems": [{ "type": "integer" }],
                            "minItems": 1,
                            "maxItems": 1
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_spec_with_static_and_reducer_endpoints() {
        // A schema with one reducer taking (name: String, count: U32).
        let schema: SchemaResponse = serde_json::from_value(json!({
            "tables": [],
            "reducers": [{
                "name": "add_player",
                "params": {
                    "elements": [
                        { "name": { "some": "name" }, "algebraic_type": { "String": [] } },
                        { "name": { "some": "count" }, "algebraic_type": { "U32": [] } }
                    ]
                }
            }]
        }))
        .unwrap();

        let spec = build_spec(
            "https://testnet.spacetimedb.com",
            "https://spacetimedb.com",
            "c200deadbeef",
            &schema,
        );

        // Top-level shape.
        assert_eq!(spec["openapi"], "3.1.0");
        let paths = spec["paths"].as_object().unwrap();

        // Static endpoints are present, with the concrete identity baked in.
        assert!(paths.contains_key("/v1/database/c200deadbeef/schema"));
        assert!(paths.contains_key("/v1/database/c200deadbeef/sql"));
        assert!(paths.contains_key("/v1/database/c200deadbeef/subscribe"));
        assert!(paths.contains_key("https://spacetimedb.com/api/spacetimedb-token"));

        // The reducer got its own concrete, typed endpoint.
        let reducer_path = "/v1/database/c200deadbeef/call/add_player";
        let body_schema = &paths[reducer_path]["post"]["requestBody"]["content"]
            ["application/json"]["schema"];
        assert_eq!(body_schema["type"], "array");
        assert_eq!(body_schema["minItems"], 2);
        assert_eq!(body_schema["prefixItems"][0]["type"], "string");
        assert_eq!(body_schema["prefixItems"][1]["type"], "integer");

        // A populated example must exist so RapiDoc's "Fill Example" works.
        let example = &paths[reducer_path]["post"]["requestBody"]["content"]
            ["application/json"]["example"];
        assert_eq!(example, &json!(["<name>", 0]));

        // Whole document must serialize.
        assert!(serde_json::to_string(&spec).is_ok());
    }
}
