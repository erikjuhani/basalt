use std::fmt;

#[derive(Default)]
pub struct VersionInfo {
    pub version: &'static str,
    pub hash: Option<&'static str>,
    pub short_hash: Option<&'static str>,
    pub date: Option<&'static str>,
}

impl VersionInfo {
    pub const fn from_env() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            hash: option_env!("BASALT_COMMIT_HASH"),
            short_hash: option_env!("BASALT_COMMIT_SHORT_HASH"),
            date: option_env!("BASALT_COMMIT_DATE"),
        }
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)?;
        match (self.short_hash, self.date) {
            (None, _) => Ok(()),
            (Some(short_hash), None) => write!(f, " ({})", short_hash),
            (Some(short_hash), Some(date)) => write!(f, " ({} {})", short_hash, date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_commit_info() {
        let info = VersionInfo {
            version: "0.12.5",
            hash: Some("abc123def0123456789"),
            short_hash: Some("abc123def"),
            date: Some("2026-05-15"),
        };
        assert_eq!(info.to_string(), "0.12.5 (abc123def 2026-05-15)");
    }

    #[test]
    fn without_commit_info() {
        let info = VersionInfo {
            version: "0.12.5",
            ..Default::default()
        };
        assert_eq!(info.to_string(), "0.12.5");
    }
}
