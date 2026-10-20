//! Public product links shared by About and OAuth client metadata.

pub const HOMEPAGE_URL: &str = "https://hagency.org/";
pub const SOURCE_URL: &str = env!("CARGO_PKG_REPOSITORY");
pub const PRIVACY_POLICY_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/crates/hagency-app/docs/privacy.md"
);
pub const LICENSE_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/crates/hagency-app/LICENSE"
);
pub const NOTICES_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/crates/hagency-app/NOTICE"
);
pub const GUIDE_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/crates/hagency-app/docs/hagency-quickstart.md"
);
pub const RELEASES_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases");
// Client is a fork with Issues disabled. Keep reporting on the open Desktop tracker.
pub const NEW_ISSUE_URL: &str = "https://github.com/hagency-org/hagency-desktop/issues/new";
