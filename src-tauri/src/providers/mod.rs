use serde::{Deserialize, Serialize};

pub mod categories;
pub mod chocolatey;
pub mod dedupe;
pub mod popular;
pub mod scoop;
pub mod winget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Chocolatey,
    Winget,
    Scoop,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chocolatey => "chocolatey",
            Self::Winget => "winget",
            Self::Scoop => "scoop",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "chocolatey" | "choco" => Some(Self::Chocolatey),
            "winget" => Some(Self::Winget),
            "scoop" => Some(Self::Scoop),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub id: String,
    pub provider: ProviderKind,
    pub name: String,
    pub version: Option<String>,
    pub available_version: Option<String>,
    pub summary: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub pinned: bool,
    pub outdated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageAction {
    Install,
    Uninstall,
    Upgrade,
    Pin,
    Unpin,
}

impl PackageAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Upgrade => "upgrade",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChocoSource {
    pub name: String,
    pub url: String,
    pub disabled: bool,
    pub priority: Option<i32>,
}

pub fn package_id(provider: ProviderKind, name: &str) -> String {
    format!("{}:{name}", provider.as_str())
}

pub fn parse_package_id(id: &str) -> Result<(ProviderKind, String), String> {
    let (provider, name) = id
        .split_once(':')
        .ok_or_else(|| format!("Invalid package id (expected provider:name): {id}"))?;
    let kind = ProviderKind::parse(provider)
        .ok_or_else(|| format!("Unknown provider in package id: {provider}"))?;
    if name.is_empty() {
        return Err(format!("Empty package name in id: {id}"));
    }
    Ok((kind, name.to_string()))
}

pub fn list_installed(include_chocolatey: bool, include_winget: bool, include_scoop: bool) -> Result<Vec<Package>, String> {
    let mut packages = Vec::new();
    if include_chocolatey {
        match chocolatey::list_installed() {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => {
                if bootstrap_available_choco() {
                    return Err(err);
                }
            }
        }
    }
    if include_winget {
        match winget::list_installed() {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => {
                if crate::bootstrap::find_winget().is_some() {
                    return Err(err);
                }
            }
        }
    }
    if include_scoop {
        match scoop::list_installed() {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => {
                if crate::bootstrap::find_scoop().is_some() {
                    return Err(err);
                }
            }
        }
    }
    packages.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.provider.as_str().cmp(b.provider.as_str()))
    });
    categories::enrich_categories(&mut packages);
    Ok(packages)
}

pub fn search(
    query: &str,
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Result<Vec<Package>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let mut errors = Vec::new();

    if include_chocolatey && bootstrap_available_choco() {
        match chocolatey::search(query) {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => errors.push(format!("Chocolatey: {err}")),
        }
    }
    if include_winget && crate::bootstrap::find_winget().is_some() {
        match winget::search(query) {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => errors.push(format!("winget: {err}")),
        }
    }
    if include_scoop && crate::bootstrap::find_scoop().is_some() {
        match scoop::search(query) {
            Ok(mut list) => packages.append(&mut list),
            Err(err) => errors.push(format!("Scoop: {err}")),
        }
    }

    if packages.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    categories::enrich_categories(&mut packages);
    Ok(packages)
}

pub fn list_popular(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
) -> Vec<Package> {
    let mut packages = popular::list_popular(include_chocolatey, include_winget, include_scoop);
    categories::enrich_categories(&mut packages);
    packages
}

pub fn list_outdated(
    include_chocolatey: bool,
    include_winget: bool,
    include_scoop: bool,
    prefer_provider: Option<String>,
    show_duplicates: bool,
) -> Result<Vec<Package>, String> {
    let mut packages = Vec::new();
    if include_chocolatey && bootstrap_available_choco() {
        packages.extend(chocolatey::list_outdated()?);
    }
    if include_winget && crate::bootstrap::find_winget().is_some() {
        packages.extend(winget::list_outdated()?);
    }
    if include_scoop && crate::bootstrap::find_scoop().is_some() {
        packages.extend(scoop::list_outdated()?);
    }

    if !show_duplicates {
        let preferred = prefer_provider
            .as_deref()
            .and_then(ProviderKind::parse)
            .unwrap_or(ProviderKind::Winget);
        packages = dedupe::dedupe_for_updates(packages, preferred);
    } else {
        packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }
    categories::enrich_categories(&mut packages);
    Ok(packages)
}

fn bootstrap_available_choco() -> bool {
    crate::bootstrap::find_choco().is_some()
}
