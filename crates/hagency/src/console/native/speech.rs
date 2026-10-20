//! In-process dictation reuses the exact owner-scoped Codex selection.
use super::super::{device_execution::DeviceIdentity, server_login::AuthorizedDevice};
use super::*;
use std::{future::Future, time::Duration};

// A speech session can outlive the server's finite (<=30s) owner proof. Renew
// within that proof's deadline while continuing media work, and pin every field
// of the authorized device. A slow/failed renewal drops the media future.
async fn with_device_proof<T, F, R, RF>(
    work: F,
    mut device: AuthorizedDevice,
    mut refresh: R,
    period: Duration,
) -> Result<(T, AuthorizedDevice), NativeError>
where
    F: Future<Output = Result<T, NativeError>>,
    R: FnMut() -> RF,
    RF: Future<Output = Result<AuthorizedDevice, NativeError>>,
{
    let pin = DeviceIdentity::from(&device);
    tokio::pin!(work);
    loop {
        device
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let deadline = device.valid_until();
        let delay = period.min(deadline.saturating_duration_since(std::time::Instant::now()) / 2);
        let renewal = async {
            tokio::time::sleep(delay).await;
            refresh().await
        };
        tokio::pin!(renewal);
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(deadline.into()) => return Err(failure(401, "owner_authorization_required")),
            fresh = &mut renewal => {
                let fresh = match fresh {
                    Ok(fresh) => fresh,
                    // ServerLogin preserves finite authority on a transport
                    // failure. Retry within the same hard deadline, never past it.
                    Err(error) if error.status == 503 => continue,
                    Err(error) => return Err(error),
                };
                fresh.bearer().map_err(|_| failure(401, "owner_authorization_required"))?;
                if DeviceIdentity::from(&fresh) != pin {return Err(failure(401, "owner_authorization_required"));}
                device = fresh;
            }
            result = &mut work => {
                device.bearer().map_err(|_| failure(401, "owner_authorization_required"))?;
                return result.map(|output| (output, device));
            }
        }
    }
}

impl NativeOwner {
    /// Receives PCM16 mono 16 kHz, returns user speech only. Never sends a Room message.
    /// Dropping this future drops and kills its isolated app-server child.
    pub async fn transcribe_speech(
        &self,
        pcm: &[u8],
        locale: &str,
        context: &str,
        probe: bool,
    ) -> Result<String, NativeError> {
        self.speech_request(
            Some(pcm),
            None,
            locale,
            context,
            probe,
            std::sync::Arc::new(|_| {}),
        )
        .await
    }

    pub async fn stream_speech(
        &self,
        audio: crate::console::speech::Audio,
        locale: &str,
        context: &str,
        updates: crate::console::speech::Updates,
    ) -> Result<String, NativeError> {
        self.speech_request(None, Some(audio), locale, context, false, updates)
            .await
    }

    async fn speech_request(
        &self,
        pcm: Option<&[u8]>,
        audio: Option<crate::console::speech::Audio>,
        locale: &str,
        context: &str,
        probe: bool,
        updates: crate::console::speech::Updates,
    ) -> Result<String, NativeError> {
        let _update_task = crate::update_guard::PROCESS
            .enter_task()
            .ok_or_else(|| failure(409, "update_in_progress"))?;
        if !crate::console::speech::local_codex_supported() {
            return Err(failure(
                501,
                crate::console::speech::CODEX_PLATFORM_UNSUPPORTED,
            ));
        }
        let identity = self.identity().await?;
        let console = &self.0.host.console;
        let root = console
            .0
            .server_login
            .state_directory()
            .ok_or(failure(503, "provider_directory_unavailable"))?;
        let paths = owner_provider::paths(
            &root,
            &identity.origin,
            &identity.issuer,
            &identity.subject,
            &identity.owner,
        )
        .map_err(|e| failure(e.status, e.code))?;
        if !paths.shared {
            return Err(failure(409, "speech_codex_local_required"));
        }
        let device = console
            .authorized_device()
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        device
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let pin = DeviceIdentity::from(&device);
        let (result, proof) = with_device_proof(
            async {
                let guarded: crate::console::speech::Updates = std::sync::Arc::new({
                    let root = root.clone();
                    let identity = identity.clone();
                    let credential_ref = paths.credential_ref.clone();
                    move |event| {
                        if owner_provider::paths(
                            &root,
                            &identity.origin,
                            &identity.issuer,
                            &identity.subject,
                            &identity.owner,
                        )
                        .is_ok_and(|p| p.shared && p.credential_ref == credential_ref)
                        {
                            updates(event);
                        }
                    }
                });
                let result = if let Some(audio) = audio {
                    owner_provider::speech::stream(&paths, audio, locale, context, probe, guarded)
                        .await
                } else {
                    owner_provider::speech::transcribe(
                        &paths,
                        pcm.unwrap_or_default(),
                        locale,
                        context,
                        probe,
                    )
                    .await
                };
                result.map_err(|e| failure(e.status, e.code))
            },
            device,
            || async {
                // Match Agent management's SDK/session recovery path as well
                // as renewing the finite device proof.
                self.sync_matrix().await?;
                console.authorized_device().await.map_err(|error| {
                    if matches!(error, super::super::Error::Unavailable) {
                        failure(503, "owner_authorization_unavailable")
                    } else {
                        failure(401, "owner_authorization_required")
                    }
                })
            },
            Duration::from_secs(5),
        )
        .await?;
        if DeviceIdentity::from(&proof) != pin {
            return Err(failure(401, "owner_authorization_required"));
        }
        // Reject a result if Agent services changed the selected account meanwhile.
        let current = owner_provider::paths(
            &root,
            &identity.origin,
            &identity.issuer,
            &identity.subject,
            &identity.owner,
        )
        .map_err(|e| failure(e.status, e.code))?;
        if current.credential_ref != paths.credential_ref {
            return Err(failure(409, "provider_profile_mismatch"));
        }
        proof
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    fn device() -> (AuthorizedDevice, Arc<AtomicBool>) {
        super::super::super::server_login::fixture_device(
            "https://example.test/",
            "owner",
            "@owner:example.test",
        )
    }
    #[tokio::test]
    async fn renews_proof_while_dictation_is_pending() {
        let (device, _) = device();
        let device = device.fixture_valid_for(Duration::from_millis(100));
        let original_deadline = device.valid_until();
        let count = AtomicUsize::new(0);
        let output = with_device_proof(
            async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                Ok("draft")
            },
            device.clone(),
            || async {
                count.fetch_add(1, Ordering::Relaxed);
                Ok(device.clone().fixture_valid_for(Duration::from_millis(100)))
            },
            Duration::from_millis(2),
        )
        .await
        .unwrap();
        assert_eq!(output.0, "draft");
        assert!(std::time::Instant::now() > original_deadline);
        assert!(count.load(Ordering::Relaxed) >= 2);
    }
    #[tokio::test]
    async fn changed_device_or_revocation_discards_late_transcript() {
        for revoke in [false, true] {
            let (device, revoked) = device();
            let error = with_device_proof(
                async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok("must be discarded")
                },
                device.clone(),
                || async {
                    if revoke {
                        revoked.store(true, Ordering::Release);
                        Ok(device.clone())
                    } else {
                        Ok(super::super::super::server_login::fixture_device(
                            "https://example.test/",
                            "other",
                            "@other:example.test",
                        )
                        .0)
                    }
                },
                Duration::from_millis(2),
            )
            .await
            .err()
            .expect("authorization must reject the transcript");
            assert_eq!(error.code, "owner_authorization_required");
        }
    }
    #[tokio::test]
    async fn a_stalled_renewal_cannot_extend_an_expired_proof() {
        let (device, _) = device();
        let device = device.fixture_valid_for(Duration::from_millis(20));
        let error = with_device_proof(
            async {
                tokio::time::sleep(Duration::from_millis(80)).await;
                Ok("too late")
            },
            device,
            || async { std::future::pending::<Result<AuthorizedDevice, NativeError>>().await },
            Duration::from_millis(2),
        )
        .await
        .err()
        .expect("expired proof must reject the transcript");
        assert_eq!(error.code, "owner_authorization_required");
    }
    #[tokio::test]
    async fn transient_failure_preserves_only_the_existing_finite_proof() {
        let (device, _) = device();
        let result = with_device_proof(
            async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok("draft")
            },
            device.clone().fixture_valid_for(Duration::from_millis(100)),
            || async { Err(failure(503, "owner_authorization_unavailable")) },
            Duration::from_millis(2),
        )
        .await
        .unwrap();
        assert_eq!(result.0, "draft");
        let error = with_device_proof(
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("too late")
            },
            device.fixture_valid_for(Duration::from_millis(20)),
            || async { Err(failure(503, "owner_authorization_unavailable")) },
            Duration::from_millis(2),
        )
        .await
        .err()
        .expect("transport failure must not extend authority");
        assert_eq!(error.code, "owner_authorization_required");
    }
}
