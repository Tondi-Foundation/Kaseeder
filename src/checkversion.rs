use anyhow::Result;
use tracing::warn;

/// Version checker
pub struct VersionChecker;

impl VersionChecker {
    /// Check if user agent version meets minimum requirements
    pub fn check_version(min_version: &str, peer_version: &str) -> Result<()> {
        if min_version.is_empty() || peer_version.is_empty() {
            return Ok(());
        }

        match Self::compare_semantic_versions(min_version, peer_version) {
            Ok(ordering) => {
                if ordering == std::cmp::Ordering::Greater {
                    return Err(anyhow::anyhow!(
                        "User agent version {} is below minimum required version {}",
                        peer_version,
                        min_version
                    ));
                }
                Ok(())
            }
            Err(e) => {
                warn!("Version comparison failed: {}. Accepting peer version.", e);
                Ok(()) // If version comparison fails, accept the version
            }
        }
    }

    /// Compare semantic versions
    fn compare_semantic_versions(version1: &str, version2: &str) -> Result<std::cmp::Ordering> {
        let v1_parts: Vec<u32> = version1
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect();

        let v2_parts: Vec<u32> = version2
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect();

        if v1_parts.is_empty() || v2_parts.is_empty() {
            return Err(anyhow::anyhow!("Invalid version format"));
        }

        // Compare version numbers
        let max_len = std::cmp::max(v1_parts.len(), v2_parts.len());

        for i in 0..max_len {
            let v1_part = v1_parts.get(i).copied().unwrap_or(0);
            let v2_part = v2_parts.get(i).copied().unwrap_or(0);

            match v1_part.cmp(&v2_part) {
                std::cmp::Ordering::Equal => continue,
                other => return Ok(other),
            }
        }

        // All parts are equal
        Ok(std::cmp::Ordering::Equal)
    }

    /// Check if protocol version meets minimum requirements
    pub fn check_protocol_version(peer_version: u32, min_version: u16) -> Result<()> {
        if min_version == 0 {
            // No minimum version requirement set
            return Ok(());
        }

        if peer_version < min_version as u32 {
            return Err(anyhow::anyhow!(
                "Protocol version {} is below minimum required version {}",
                peer_version,
                min_version
            ));
        }

        // Check if version is within reasonable range
        if peer_version > 100 {
            return Err(anyhow::anyhow!(
                "Protocol version {} seems unreasonably high",
                peer_version
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        // Test basic version comparison
        assert!(VersionChecker::check_version("1.0.0", "1.0.1").is_ok());
        assert!(VersionChecker::check_version("1.0.1", "1.0.0").is_err());
        assert!(VersionChecker::check_version("1.0.0", "1.0.0").is_ok());
    }

    #[test]
    fn test_protocol_version_check() {
        assert!(VersionChecker::check_protocol_version(5, 4).is_ok());
        assert!(VersionChecker::check_protocol_version(3, 4).is_err());
        assert!(VersionChecker::check_protocol_version(5, 0).is_ok()); // No minimum version requirement
    }

    #[test]
    fn test_semantic_version_comparison() {
        let result = VersionChecker::compare_semantic_versions("1.2.3", "1.2.4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::cmp::Ordering::Less);

        let result = VersionChecker::compare_semantic_versions("2.0.0", "1.9.9");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::cmp::Ordering::Greater);
    }
}
