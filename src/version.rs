use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_commit: String,
    pub build_date: String,
    pub rust_version: String,
}

impl VersionInfo {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: "unknown".to_string(), // Remove dependency on VERGEN_GIT_SHA
            build_date: chrono::Utc::now().to_rfc3339(), // Use current time
            rust_version: "unknown".to_string(), // Remove dependency on VERGEN_RUSTC_SEMVER
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn git_commit(&self) -> &str {
        &self.git_commit
    }

    pub fn build_date(&self) -> &str {
        &self.build_date
    }

    pub fn rust_version(&self) -> &str {
        &self.rust_version
    }

    pub fn to_string(&self) -> String {
        format!(
            "kaseeder v{} (commit: {}, built: {}, rust: {})",
            self.version, self.git_commit, self.build_date, self.rust_version
        )
    }
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: "unknown".to_string(),
            build_date: "unknown".to_string(),
            rust_version: "unknown".to_string(),
        }
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn version_info() -> VersionInfo {
    VersionInfo::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_creation() {
        let version_info = VersionInfo::new();
        assert!(!version_info.version.is_empty());
        assert!(!version_info.git_commit.is_empty());
        assert!(!version_info.build_date.is_empty());
        assert!(!version_info.rust_version.is_empty());
    }

    #[test]
    fn test_version_function() {
        let version_str = version();
        assert!(!version_str.is_empty());
        assert_eq!(version_str, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_version_info_to_string() {
        let version_info = VersionInfo::new();
        let version_string = version_info.to_string();
        assert!(version_string.contains("kaseeder"));
        assert!(version_string.contains(&version_info.version));
    }
}
