//! Bundle identity and immutable release configuration, checked before any updater starts.
use base64::Engine;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub(super) const PRODUCT_ID: &str = "org.hagency.desktop";
pub(super) const PROTOTYPE_ID: &str = "org.hagency.desktop.sparkle-prototype";

#[derive(Debug, Deserialize)]
pub(super) struct Config {
    pub identifier: String,
    #[serde(default)]
    pub root: PathBuf,
    pub bundle: PathBuf,
    pub version: String,
    pub feed: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub team_id: String,
    #[serde(default)]
    pub developer_id_signed: bool,
    #[serde(default)]
    pub signal_only: bool,
    pub automatic_updates_allowed: Option<bool>,
    pub automatically_update: Option<bool>,
}

pub(super) struct BuildTrust<'a> {
    pub packaged_release: bool,
    pub public_key: &'a str,
    pub team_id: &'a str,
    pub arch: &'a str,
    pub version: &'a str,
}

impl Config {
    pub fn prototype(&self) -> bool {
        self.identifier == PROTOTYPE_ID
    }

    pub fn validate_product(&self, trust: &BuildTrust<'_>) -> anyhow::Result<()> {
        anyhow::ensure!(
            trust.packaged_release,
            "Sparkle requires a packaged release build"
        );
        anyhow::ensure!(
            self.identifier == PRODUCT_ID && self.enabled,
            "Not a Sparkle product bundle"
        );
        anyhow::ensure!(
            self.bundle.extension().is_some_and(|value| value == "app"),
            "Not an app bundle"
        );
        anyhow::ensure!(
            !trust.team_id.is_empty() && self.developer_id_signed && self.team_id == trust.team_id,
            "Developer ID signature does not match the configured team"
        );
        anyhow::ensure!(
            !trust.public_key.is_empty()
                && self.public_key == trust.public_key
                && base64::engine::general_purpose::STANDARD
                    .decode(&self.public_key)?
                    .len()
                    == 32,
            "Sparkle public key does not match this build"
        );
        anyhow::ensure!(
            self.version == trust.version && hagency_updater::stable_version(&self.version).is_ok(),
            "Sparkle requires the stable Cargo bundle version"
        );
        let arch = match trust.arch {
            "aarch64" => "arm64",
            "x86_64" => "amd64",
            _ => anyhow::bail!("Unsupported Sparkle architecture"),
        };
        anyhow::ensure!(
            self.feed
                == format!(
                    "https://github.com/hagency-org/hagency-client/releases/latest/download/appcast-macos-{arch}.xml"
                ),
            "Unexpected Sparkle feed for this executable architecture"
        );
        anyhow::ensure!(
            self.automatic_updates_allowed == Some(false)
                && self.automatically_update == Some(false),
            "Unattended installation must be disabled"
        );
        Ok(())
    }

    pub fn validate_executable(&self, executable: &Path) -> anyhow::Result<()> {
        anyhow::ensure!(
            executable.canonicalize()?
                == self.bundle.join("Contents/MacOS/hagency").canonicalize()?,
            "Unexpected bundle executable"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Config, BuildTrust<'static>) {
        let key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        (Config {
            identifier: PRODUCT_ID.into(), root: PathBuf::new(), bundle: "/Applications/Hagency.app".into(),
            version: "0.7.0".into(),
            feed: "https://github.com/hagency-org/hagency-client/releases/latest/download/appcast-macos-arm64.xml".into(),
            enabled: true, public_key: key.into(), team_id: "TESTTEAM01".into(), developer_id_signed: true, signal_only: false,
            automatic_updates_allowed: Some(false), automatically_update: Some(false),
        }, BuildTrust { packaged_release: true, public_key: key, team_id: "TESTTEAM01", arch: "aarch64", version: "0.7.0" })
    }

    #[test]
    fn only_the_configured_signed_product_can_use_its_architecture_feed() {
        let (mut config, mut trust) = fixture();
        config.validate_product(&trust).unwrap();
        trust.arch = "x86_64";
        assert!(
            config.validate_product(&trust).is_err(),
            "Rosetta follows executable architecture"
        );
        config.feed = config.feed.replace("arm64", "amd64");
        config.validate_product(&trust).unwrap();
        trust.packaged_release = false;
        assert!(config.validate_product(&trust).is_err());
    }

    #[test]
    fn absent_or_mismatched_release_configuration_is_refused() {
        for kind in [
            "identity", "disabled", "unsigned", "team", "key", "feed", "version", "preview",
            "auto", "missing",
        ] {
            let (mut config, mut trust) = fixture();
            match kind {
                "identity" => config.identifier.push_str(".development"),
                "disabled" => config.enabled = false,
                "unsigned" => config.developer_id_signed = false,
                "team" => trust.team_id = "OTHERTEAM1",
                "key" => trust.public_key = "",
                "feed" => config.feed.push_str("?override=1"),
                "version" => config.version = "0.8.0".into(),
                "preview" => {
                    config.version = "0.7.0-beta.1".into();
                    trust.version = "0.7.0-beta.1";
                }
                "auto" => config.automatic_updates_allowed = Some(true),
                "missing" => config.automatically_update = None,
                _ => unreachable!(),
            }
            assert!(config.validate_product(&trust).is_err(), "{kind}");
        }
    }
}
