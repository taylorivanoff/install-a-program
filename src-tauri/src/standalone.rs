use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
#[cfg(not(feature = "embedded-runner"))]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const BUNDLE_MAGIC: &[u8; 10] = b"IAPBUNDLE\x01";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandaloneBundle {
    pub v: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub ids: Vec<String>,
}

impl StandaloneBundle {
    pub fn new(name: Option<String>, ids: Vec<String>) -> Self {
        Self {
            v: 1,
            name,
            ids,
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Failed to encode bundle: {e}"))
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let bundle: Self =
            serde_json::from_str(text).map_err(|e| format!("Invalid bundle JSON: {e}"))?;
        if bundle.v != 1 {
            return Err(format!("Unsupported bundle version: {}", bundle.v));
        }
        if bundle.ids.is_empty() {
            return Err("Bundle has no package ids".into());
        }
        for id in &bundle.ids {
            if !id.starts_with("winget:") {
                return Err(format!(
                    "Standalone installer supports winget packages only (got {id})"
                ));
            }
        }
        Ok(bundle)
    }
}

pub fn embed_bundle(template: &[u8], bundle: &StandaloneBundle) -> Result<Vec<u8>, String> {
    let json = bundle.to_json()?;
    let json_bytes = json.as_bytes();
    if json_bytes.len() > u32::MAX as usize {
        return Err("Bundle payload is too large".into());
    }

    let mut out = Vec::with_capacity(template.len() + BUNDLE_MAGIC.len() + 4 + json_bytes.len());
    out.extend_from_slice(template);
    out.extend_from_slice(BUNDLE_MAGIC);
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(json_bytes);
    Ok(out)
}

pub fn extract_bundle_from_bytes(data: &[u8]) -> Result<StandaloneBundle, String> {
    let magic_len = BUNDLE_MAGIC.len();
    if data.len() < magic_len + 4 {
        return Err("No embedded bundle found".into());
    }

    let Some(start) = data.windows(magic_len).rposition(|window| window == BUNDLE_MAGIC) else {
        return Err("No embedded bundle found".into());
    };

    let len_start = start + magic_len;
    let len_end = len_start + 4;
    let json_len = u32::from_le_bytes(
        data[len_start..len_end]
            .try_into()
            .map_err(|_| "Invalid bundle length".to_string())?,
    ) as usize;

    let json_start = len_end;
    let json_end = json_start
        .checked_add(json_len)
        .ok_or_else(|| "Invalid bundle length".to_string())?;
    if json_end > data.len() {
        return Err("Truncated bundle payload".into());
    }

    let json = std::str::from_utf8(&data[json_start..json_end])
        .map_err(|e| format!("Bundle payload is not UTF-8: {e}"))?;
    StandaloneBundle::from_json(json)
}

pub fn extract_bundle_from_path(path: &Path) -> Result<StandaloneBundle, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    extract_bundle_from_bytes(&data)
}

#[cfg(feature = "embedded-runner")]
pub fn runner_template_bytes() -> Result<Vec<u8>, String> {
    Ok(include_bytes!(concat!(env!("OUT_DIR"), "/runner-template.exe")).to_vec())
}

#[cfg(not(feature = "embedded-runner"))]
pub fn runner_template_bytes() -> Result<Vec<u8>, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("resources/runner-template.exe"),
        manifest_dir.join("target/release/install-a-program-runner.exe"),
        manifest_dir.join("target/debug/install-a-program-runner.exe"),
    ];

    for path in candidates {
        if path.is_file() {
            return fs::read(&path)
                .map_err(|e| format!("Failed to read runner template {}: {e}", path.display()));
        }
    }

    Err(
        "Standalone runner template not found. Build install-a-program-runner first (npm run build:runner)."
            .into(),
    )
}

pub fn export_standalone_installer(dest_path: &Path, bundle: &StandaloneBundle) -> Result<usize, String> {
    let template = runner_template_bytes()?;
    let bytes = embed_bundle(&template, bundle)?;
    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
    }
    let mut file = File::create(dest_path)
        .map_err(|e| format!("Failed to create {}: {e}", dest_path.display()))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write {}: {e}", dest_path.display()))?;
    Ok(bundle.ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_template_is_embedded() {
        let bytes = runner_template_bytes().expect("embedded runner template");
        assert!(bytes.len() > 1000, "runner template should be a real PE");
    }

    #[test]
    fn embed_and_extract_roundtrip() {
        let template = runner_template_bytes().unwrap();
        let bundle = StandaloneBundle::new(
            Some("test".into()),
            vec!["winget:Google.Chrome".into()],
        );
        let embedded = embed_bundle(&template, &bundle).unwrap();
        let extracted = extract_bundle_from_bytes(&embedded).unwrap();
        assert_eq!(extracted.name, bundle.name);
        assert_eq!(extracted.ids, bundle.ids);
    }
}
