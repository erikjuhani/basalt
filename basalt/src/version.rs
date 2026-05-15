use std::fmt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VersionInfo {
    pub version: &'static str,
    pub short_hash: &'static str,
    pub date: &'static str,
}

impl VersionInfo {
    pub const fn from_env() -> Self {
        Self {
            version: env!("BASALT_VERSION"),
            short_hash: env!("BASALT_COMMIT_SHORT_HASH"),
            date: env!("BASALT_COMMIT_DATE"),
        }
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({} {})", self.version, self.short_hash, self.date)
    }
}

#[test]
fn version_format() {
    let info = VersionInfo {
        version: "0.12.5",
        short_hash: "abc123def",
        date: "2026-05-15",
    };
    assert_eq!(info.to_string(), "0.12.5 (abc123def 2026-05-15)");
}
