//! Shared helpers for exporting tabular result sets to disk.
//!
//! Used by both the table browser and the SQL console so the CSV/JSON
//! formatting and the native save dialog live in one place.

use serde_json::Value;

/// Escape a single CSV field per RFC 4180.
pub fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

/// Build a CSV document from a header and pre-formatted string rows.
///
/// Callers stringify their cells with their own `format_cell` first, so the
/// table-specific value rendering (timestamps, identities, …) stays local.
pub fn build_csv(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();

    // Header row
    let header: Vec<String> = columns.iter().map(|c| escape_csv_field(c)).collect();
    out.push_str(&header.join(","));
    out.push('\n');

    // Data rows
    for row in rows {
        let fields: Vec<String> = row.iter().map(|f| escape_csv_field(f)).collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }

    out
}

/// Build a pretty-printed JSON document (an array of column-keyed objects)
/// from positional raw values.
pub fn build_json(columns: &[String], rows: &[Vec<Value>]) -> String {
    let objects: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                obj.insert(col.clone(), row.get(i).cloned().unwrap_or(Value::Null));
            }
            Value::Object(obj)
        })
        .collect();

    serde_json::to_string_pretty(&Value::Array(objects)).unwrap_or_default()
}

/// Save text content to disk via the native save dialog.
pub async fn save_text_file(file_name: &str, filter_name: &str, extensions: &[&str], content: &str) {
    let file_handle = rfd::AsyncFileDialog::new()
        .set_file_name(file_name)
        .add_filter(filter_name, extensions)
        .save_file()
        .await;

    if let Some(handle) = file_handle {
        if let Err(e) = handle.write(content.as_bytes()).await {
            log::error!("Failed to write export: {e}");
        } else {
            log::info!("Exported file to {:?}", handle.file_name());
        }
    }
}
