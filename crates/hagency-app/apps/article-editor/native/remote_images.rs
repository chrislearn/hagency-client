//! Host-controlled HTTP(S) image loading. Renderers receive bounded PNG bytes only.
use article_makepad::content::{Images, prepare_image};
use futures_util::{stream, StreamExt};
use matrix_sdk::reqwest::redirect::Policy;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

fn allowed(url: &url::Url) -> bool {
    if !matches!(url.scheme(), "https" | "http") || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match url.host() {
        Some(url::Host::Domain(host)) => {
            host.contains('.')
                && host != "localhost"
                && !host.ends_with(".local")
                && !host.ends_with(".localhost")
        }
        Some(url::Host::Ipv4(ip)) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_unspecified()
                && !ip.is_multicast()
        }
        Some(url::Host::Ipv6(ip)) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
        }
        None => false,
    }
}

pub async fn load(urls: Vec<String>) -> Images {
    let Ok(client) = crate::http::client_builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Rinx article preview")
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !allowed(attempt.url()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
    else {
        return Images::default();
    };
    let mut jobs = stream::iter(urls.into_iter().take(article_core::document::MAX_IMAGES).map(|url| {
        let client = client.clone();
        async move {
            let parsed = url::Url::parse(&url).ok().filter(allowed)?;
            let mut response = client
                .get(parsed)
                .send()
                .await
                .ok()?
                .error_for_status()
                .ok()?;
            if response
                .content_length()
                .is_some_and(|n| n > 8 * 1024 * 1024)
            {
                return None;
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.ok()? {
                if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            let image = tokio::task::spawn_blocking(move || prepare_image(&bytes))
                .await
                .ok()?
                .ok()?;
            Some((url, image))
        }
    }))
    .buffer_unordered(4);
    let mut images = BTreeMap::new();
    let mut total = 0;
    while let Some(result) = jobs.next().await {
        let Some((url, image)) = result else {
            continue;
        };
        if total + image.png.len() <= 20 * 1024 * 1024 {
            total += image.png.len();
            images.insert(url, image);
        }
    }
    Arc::new(images)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_fetch_has_no_local_or_credential_urls() {
        for url in [
            "file:///etc/passwd",
            "https://127.0.0.1/a.png",
            "https://localhost/a",
            "https://app.local/a",
            "https://user:password@example.org/a",
            "https://[::1]/a",
        ] {
            assert!(!allowed(&url::Url::parse(url).unwrap()), "{url}");
        }
        assert!(allowed(
            &url::Url::parse(article_core::render::EDITORMD_LOGO).unwrap()
        ));
    }
}
