//! Desktop notifications use public release metadata and open official downloads.
//! This metadata never authorizes executing or replacing an application.
use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::Deserialize;
use std::{io::Read, time::Duration};

const ENDPOINT: &str = "https://api.github.com/repos/hagency-org/hagency-client/releases/latest";
const MAX_RELEASE: u64 = 512 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum DesktopPlatform {
    MacArm64,
    MacAmd64,
    LinuxArm64,
    LinuxAmd64,
}

impl DesktopPlatform {
    fn asset(self) -> &'static str {
        match self {
            Self::MacArm64 => "hagency-macos-arm64.dmg",
            Self::MacAmd64 => "hagency-macos-amd64.dmg",
            Self::LinuxArm64 => "hagency-linux-arm64.deb",
            Self::LinuxAmd64 => "hagency-linux-amd64.deb",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub notes: String,
    pub download_url: String,
}

#[derive(Deserialize)]
struct Metadata {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    state: String,
}

pub fn parse_release(
    bytes: &[u8],
    current: &Version,
    architecture: DesktopPlatform,
) -> Result<Option<Release>> {
    ensure!(
        bytes.len() as u64 <= MAX_RELEASE,
        "Release metadata is too large"
    );
    let metadata: Metadata = serde_json::from_slice(bytes)?;
    if metadata.draft || metadata.prerelease {
        return Ok(None);
    }
    let version = Version::parse(
        metadata
            .tag_name
            .strip_prefix('v')
            .context("Invalid release tag")?,
    )?;
    if !version.pre.is_empty() || version <= *current {
        return Ok(None);
    }
    let mut assets = metadata
        .assets
        .iter()
        .filter(|asset| asset.name == architecture.asset());
    let asset = assets
        .next()
        .context("No update is available for this desktop platform")?;
    ensure!(assets.next().is_none(), "Ambiguous update asset");
    let expected = format!(
        "https://github.com/hagency-org/hagency-client/releases/download/v{version}/{}",
        architecture.asset()
    );
    ensure!(
        asset.browser_download_url == expected,
        "Unexpected update download URL"
    );
    ensure!(
        asset.state == "uploaded" && asset.size > 0 && asset.size <= crate::MAX_INSTALLER,
        "Invalid update asset"
    );
    let notes = metadata.body.unwrap_or_default();
    ensure!(notes.len() <= 64 * 1024, "Release notes are too large");
    let download_url = match architecture {
        DesktopPlatform::LinuxArm64 | DesktopPlatform::LinuxAmd64 => {
            format!("https://github.com/hagency-org/hagency-client/releases/tag/v{version}")
        }
        _ => expected,
    };
    Ok(Some(Release {
        version,
        notes,
        download_url,
    }))
}

pub struct ReleaseClient(reqwest::blocking::Client);

impl ReleaseClient {
    pub fn new() -> Result<Self> {
        Ok(Self(
            reqwest::blocking::Client::builder()
                .https_only(true)
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(concat!("Hagency-Update-Check/", env!("CARGO_PKG_VERSION")))
                .build()?,
        ))
    }

    pub fn check(
        &self,
        current: &Version,
        architecture: DesktopPlatform,
    ) -> Result<Option<Release>> {
        self.check_url(ENDPOINT, current, architecture)
    }

    fn check_url(
        &self,
        endpoint: &str,
        current: &Version,
        architecture: DesktopPlatform,
    ) -> Result<Option<Release>> {
        let response = self
            .0
            .get(endpoint)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .send()?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response.error_for_status()?;
        let mut bytes = Vec::new();
        response.take(MAX_RELEASE + 1).read_to_end(&mut bytes)?;
        parse_release(&bytes, current, architecture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata() -> serde_json::Value {
        json!({"tag_name":"v0.7.0", "draft":false, "prerelease":false, "body":"Release notes",
        "assets": (["arm64", "amd64"].map(|arch| json!({
            "name":format!("hagency-macos-{arch}.dmg"), "size":100, "state":"uploaded",
            "browser_download_url":format!("https://github.com/hagency-org/hagency-client/releases/download/v0.7.0/hagency-macos-{arch}.dmg")
        })))})
    }

    fn parse(
        metadata: &serde_json::Value,
        architecture: DesktopPlatform,
    ) -> Result<Option<Release>> {
        parse_release(
            &serde_json::to_vec(metadata)?,
            &Version::new(0, 6, 0),
            architecture,
        )
    }

    #[test]
    fn macos_release_selects_the_matching_dmg_and_ignores_old_or_preview_versions() {
        for (architecture, suffix) in [
            (DesktopPlatform::MacArm64, "arm64.dmg"),
            (DesktopPlatform::MacAmd64, "amd64.dmg"),
        ] {
            let release = parse(&metadata(), architecture).unwrap().unwrap();
            assert!(release.download_url.ends_with(suffix));
            assert_eq!(release.notes, "Release notes");
        }
        for (key, value) in [
            ("draft", json!(true)),
            ("prerelease", json!(true)),
            ("tag_name", json!("v0.6.0")),
            ("tag_name", json!("v0.5.0")),
            ("tag_name", json!("v0.8.0-rc.1")),
        ] {
            let mut metadata = metadata();
            metadata[key] = value;
            assert!(
                parse(&metadata, DesktopPlatform::MacArm64)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn macos_release_refuses_missing_incomplete_or_foreign_downloads() {
        for (key, value) in [
            (
                "browser_download_url",
                json!("https://example.com/hagency.dmg"),
            ),
            (
                "browser_download_url",
                json!(
                    "https://github.com/hagency-org/hagency-client/releases/download/v0.6.0/hagency-macos-arm64.dmg"
                ),
            ),
            ("size", json!(0)),
            ("state", json!("new")),
        ] {
            let mut metadata = metadata();
            metadata["assets"][0][key] = value;
            assert!(parse(&metadata, DesktopPlatform::MacArm64).is_err());
        }
        let mut metadata = metadata();
        metadata["assets"] = json!([]);
        assert!(parse(&metadata, DesktopPlatform::MacArm64).is_err());
        assert!(
            parse_release(
                &vec![b' '; MAX_RELEASE as usize + 1],
                &Version::new(0, 6, 0),
                DesktopPlatform::MacArm64
            )
            .is_err()
        );
    }

    #[test]
    fn linux_release_requires_the_matching_package_and_opens_the_version_download_page() {
        for (platform, arch) in [
            (DesktopPlatform::LinuxAmd64, "amd64"),
            (DesktopPlatform::LinuxArm64, "arm64"),
        ] {
            let mut metadata = metadata();
            metadata["assets"] = json!([{
                "name": format!("hagency-linux-{arch}.deb"), "state":"uploaded", "size":100,
                "browser_download_url":format!("https://github.com/hagency-org/hagency-client/releases/download/v0.7.0/hagency-linux-{arch}.deb")
            }]);
            let release = parse(&metadata, platform).unwrap().unwrap();
            assert_eq!(
                release.download_url,
                "https://github.com/hagency-org/hagency-client/releases/tag/v0.7.0"
            );
            assert!(parse(&metadata, DesktopPlatform::MacArm64).is_err());
            metadata["assets"][0]["browser_download_url"] = json!("https://example.org/download");
            assert!(parse(&metadata, platform).is_err());
        }
        assert!(parse(&metadata(), DesktopPlatform::LinuxArm64).is_err());
    }

    #[test]
    fn macos_release_http_reports_api_failures_and_handles_a_repository_without_releases() {
        use std::{io::Write, net::TcpListener};
        for (status, body, success) in [
            (200, metadata().to_string(), true),
            (404, "{}".into(), true),
            (403, "{}".into(), false),
            (200, "invalid".into(), false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/latest", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).unwrap();
                    assert!(count > 0 && request.len() + count <= 8192);
                    request.extend_from_slice(&buffer[..count]);
                }
                let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
                assert!(request.contains("x-github-api-version: 2026-03-10"));
                assert!(!request.contains("authorization:"));
                write!(socket, "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            // Only this isolated fixture uses HTTP; production has a fixed HTTPS endpoint.
            let client = ReleaseClient(
                reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .no_proxy()
                    .build()
                    .unwrap(),
            );
            let result =
                client.check_url(&endpoint, &Version::new(0, 6, 0), DesktopPlatform::MacArm64);
            assert_eq!(result.is_ok(), success);
            if status == 404 {
                assert!(result.unwrap().is_none());
            }
            server.join().unwrap();
        }
    }
}
