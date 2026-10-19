//! Stable, credential-free classification of Matrix/OAuth authorization failures.
use matrix_sdk::{
    HttpError, RefreshTokenError,
    authentication::oauth::error::{OAuthError, RequestTokenError, BasicErrorResponseType},
};

pub(crate) fn refresh_requires_login(error: &RefreshTokenError) -> bool {
    match error {
        RefreshTokenError::RefreshTokenRequired => true,
        RefreshTokenError::OAuth(error) => match error.as_ref() {
            OAuthError::NotRegistered | OAuthError::NotAuthenticated => true,
            OAuthError::RefreshToken(RequestTokenError::ServerResponse(response)) => matches!(
                response.error(),
                BasicErrorResponseType::InvalidGrant
                    | BasicErrorResponseType::InvalidClient
                    | BasicErrorResponseType::UnauthorizedClient
            ),
            _ => false,
        },
        RefreshTokenError::MatrixAuth(error) => http_requires_login(error),
    }
}
pub(crate) fn http_requires_login(error: &HttpError) -> bool {
    use matrix_sdk::ruma::api::error::ErrorKind;
    matches!(
        error.client_api_error_kind(),
        Some(ErrorKind::UnknownToken(..) | ErrorKind::MissingToken)
    ) || matches!(error, HttpError::RefreshToken(error) if refresh_requires_login(error))
}
pub(crate) fn requires_login(error: &matrix_sdk::Error) -> bool {
    matches!(error, matrix_sdk::Error::Http(error) if http_requires_login(error))
}
pub(crate) fn refresh_failure_code(error: &RefreshTokenError) -> &'static str {
    if refresh_requires_login(error) {
        "matrix_authorization_required"
    } else {
        "matrix_authorization_unavailable"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oauth_terminal_errors_require_login_and_server_errors_remain_retryable() {
        for (code, terminal) in [
            ("invalid_grant", true),
            ("invalid_client", true),
            ("unauthorized_client", true),
            ("server_error", false),
            ("temporarily_unavailable", false),
        ] {
            let response = serde_json::from_value(serde_json::json!({"error":code,
                "error_description":"credential text must never be logged"}))
            .unwrap();
            let error = RefreshTokenError::OAuth(std::sync::Arc::new(OAuthError::RefreshToken(
                RequestTokenError::ServerResponse(response),
            )));
            assert_eq!(refresh_requires_login(&error), terminal);
            assert_eq!(
                http_requires_login(&HttpError::RefreshToken(error.clone())),
                terminal
            );
            assert_eq!(
                refresh_failure_code(&error),
                if terminal {
                    "matrix_authorization_required"
                } else {
                    "matrix_authorization_unavailable"
                }
            );
        }
    }
}
