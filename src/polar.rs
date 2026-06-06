//! Self-contained [Polar](https://polar.sh) license-activation logic.
//!
//! This module is intentionally **UI-free and dependency-light** so it can be
//! lifted into other applications unchanged. The entire public surface is two
//! async functions:
//!
//! ```ignore
//! let active = polar::check_license(key).await; // activate this device if needed, then validate
//! polar::disable_license(key).await;            // release this device's activation
//! ```
//!
//! Everything else — the Polar HTTP contract, this device's identity, and the
//! local activation bookkeeping — is an internal implementation detail.
//!
//! ## How it works
//!
//! Polar's *customer-portal* license endpoints are public (no auth token); they
//! only need the organization id and the license key, so they are safe to call
//! directly from a desktop client. A license may permit a limited number of
//! device *activations*; we activate this device once and remember the returned
//! `activation_id` locally so later validation/deactivation can target it.
//!
//! To reuse in another app, copy this file and set the four constants below
//! (base URL is environment-derived; the organization id must be filled in).

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::json;

// --- Environment configuration ---------------------------------------------
//
// Debug builds (`dx serve`) talk to Polar's sandbox; release builds
// (`dx build --release`) talk to production. `cfg!(debug_assertions)` is the
// switch: it is on for debug and off for release.

#[cfg(debug_assertions)]
const POLAR_BASE_URL: &str = "https://sandbox-api.polar.sh";
#[cfg(not(debug_assertions))]
const POLAR_BASE_URL: &str = "https://api.polar.sh";

// TODO: fill in the real Polar organization id (UUID) for each environment.
#[cfg(debug_assertions)]
const POLAR_ORG_ID: &str = "0e5cb33e-71f4-4002-87cd-667b9935d564";
#[cfg(not(debug_assertions))]
const POLAR_ORG_ID: &str = "TODO_PRODUCTION_ORGANIZATION_ID";

/// Public checkout link where users purchase a license, opened from the
/// activation UI.
// TODO: replace with your real Polar checkout link.
#[cfg(debug_assertions)]
pub const CHECKOUT_URL: &str = "https://sandbox-api.polar.sh/v1/checkout-links/polar_cl_Km5XrftpLM7BRHDOJJLfsB4brBCniM5N1uT851cQrPJ/redirect";
#[cfg(not(debug_assertions))]
pub const CHECKOUT_URL: &str = "https://polar.sh/TODO_CHECKOUT_LINK";

// --- Public API -------------------------------------------------------------

/// Ensure the given license is active **on this device** and still valid.
///
/// Activates this device if it has not been activated yet, otherwise validates
/// the existing activation. Returns `true` only when Polar confirms the license
/// is active for this device.
///
/// Network failures are treated leniently for an *already-activated* device
/// (returns `true`, an offline grace period) but strictly for a device that has
/// never activated (returns `false`).
pub async fn check_license(license_key: &str) -> bool {
    let key = license_key.trim();
    if key.is_empty() {
        return false;
    }

    // If this device already has an activation, validate it.
    if let Some(activation_id) = load_activation(key) {
        match validate(key, &activation_id).await {
            Validation::Valid => return true,
            // Already activated but currently offline: keep working.
            Validation::Network => return true,
            // Revoked / expired / unknown activation: drop it and re-activate below.
            Validation::Invalid => remove_activation(key),
        }
    }

    // No (valid) activation yet — activate this device.
    match activate(key).await {
        Some(activation_id) => {
            store_activation(key, &activation_id);
            true
        }
        None => false,
    }
}

/// Release this device's activation for the given license.
///
/// Best-effort: it always clears the local activation record, and additionally
/// asks Polar to free the seat if we still know the activation id.
pub async fn disable_license(license_key: &str) {
    let key = license_key.trim();
    if key.is_empty() {
        return;
    }
    if let Some(activation_id) = load_activation(key) {
        let _ = deactivate(key, &activation_id).await;
    }
    remove_activation(key);
}

// --- Polar HTTP calls -------------------------------------------------------

enum Validation {
    Valid,
    Invalid,
    Network,
}

/// `POST /v1/customer-portal/license-keys/activate` — returns the new
/// `activation_id` on success, or `None` for any failure (limit reached,
/// invalid key, network error, …).
async fn activate(key: &str) -> Option<String> {
    let body = json!({
        "key": key,
        "organization_id": POLAR_ORG_ID,
        "label": device_label(),
    });
    let resp = client()
        .post(format!("{POLAR_BASE_URL}/v1/customer-portal/license-keys/activate"))
        .json(&body)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        log::warn!("Polar activation rejected: HTTP {}", resp.status());
        return None;
    }
    let parsed: ActivationResponse = resp.json().await.ok()?;
    Some(parsed.id)
}

/// `POST /v1/customer-portal/license-keys/validate` for a specific activation.
async fn validate(key: &str, activation_id: &str) -> Validation {
    let body = json!({
        "key": key,
        "organization_id": POLAR_ORG_ID,
        "activation_id": activation_id,
    });
    let resp = match client()
        .post(format!("{POLAR_BASE_URL}/v1/customer-portal/license-keys/validate"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            log::warn!("Polar validation network error: {e}");
            return Validation::Network;
        }
    };
    if !resp.status().is_success() {
        return Validation::Invalid;
    }
    match resp.json::<ValidatedLicenseKey>().await {
        Ok(parsed) if parsed.is_active() => Validation::Valid,
        _ => Validation::Invalid,
    }
}

/// `POST /v1/customer-portal/license-keys/deactivate` — frees the seat.
async fn deactivate(key: &str, activation_id: &str) -> bool {
    let body = json!({
        "key": key,
        "organization_id": POLAR_ORG_ID,
        "activation_id": activation_id,
    });
    client()
        .post(format!("{POLAR_BASE_URL}/v1/customer-portal/license-keys/deactivate"))
        .json(&body)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

#[derive(Deserialize)]
struct ActivationResponse {
    id: String,
}

#[derive(Deserialize)]
struct ValidatedLicenseKey {
    #[serde(default)]
    status: String,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    activation: Option<serde_json::Value>,
}

impl ValidatedLicenseKey {
    /// A license is usable when it is granted, not expired, and the activation
    /// we asked about actually exists.
    fn is_active(&self) -> bool {
        if self.status != "granted" || self.activation.is_none() {
            return false;
        }
        if let Some(expires_at) = &self.expires_at
            && let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at)
            && expiry < chrono::Utc::now()
        {
            return false;
        }
        true
    }
}

// --- This device's identity -------------------------------------------------

/// A human-readable, hardware-derived label shown in the Polar dashboard's
/// activation list. Combines the hostname with a stable machine id so two
/// machines that share a hostname remain distinguishable.
fn device_label() -> String {
    let host = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
    match machine_uid::get() {
        Ok(id) => format!("{host} ({})", &id[..id.len().min(8)]),
        Err(_) => host,
    }
}

// --- Local activation bookkeeping -------------------------------------------
//
// We persist a small `{ hashed_license_key -> activation_id }` map next to the
// OS data dir, namespaced by organization so multiple Polar-backed apps never
// collide. The license key itself is hashed (never written in the clear here).

fn state_path() -> Option<PathBuf> {
    Some(
        dirs::data_dir()?
            .join("polar")
            .join(POLAR_ORG_ID)
            .join("activations.json"),
    )
}

fn key_hash(license_key: &str) -> String {
    blake3::hash(license_key.as_bytes()).to_hex().to_string()
}

fn load_all() -> HashMap<String, String> {
    state_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_all(map: &HashMap<String, String>) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(map)
        && let Err(e) = std::fs::write(&path, serialized)
    {
        log::error!("Failed to persist Polar activation state: {e}");
    }
}

fn load_activation(license_key: &str) -> Option<String> {
    load_all().get(&key_hash(license_key)).cloned()
}

fn store_activation(license_key: &str, activation_id: &str) {
    let mut map = load_all();
    map.insert(key_hash(license_key), activation_id.to_string());
    save_all(&map);
}

fn remove_activation(license_key: &str) {
    let mut map = load_all();
    if map.remove(&key_hash(license_key)).is_some() {
        save_all(&map);
    }
}
