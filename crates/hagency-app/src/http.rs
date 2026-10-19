//! TLS configuration shared by Rinx's auxiliary HTTP clients.

#[cfg(target_os = "android")]
pub(crate) fn android_root_certificates() -> Vec<matrix_sdk::reqwest::Certificate> {
    webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .filter_map(|der| matrix_sdk::reqwest::Certificate::from_der(der.as_ref()).ok())
        .collect()
}

pub(crate) fn client_builder() -> matrix_sdk::reqwest::ClientBuilder {
    let builder = matrix_sdk::reqwest::Client::builder();
    // reqwest 0.13 defaults to a JVM-backed verifier on Android. Makepad
    // does not initialize that verifier; use the same bundled trust roots
    // as the Matrix SDK client, with certificate/hostname checks enabled.
    #[cfg(target_os = "android")]
    let builder = builder.tls_certs_only(android_root_certificates());
    builder
}
