use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tracing::warn;

/// JWT claims for service-to-service authentication.
#[derive(Debug, Deserialize)]
struct Claims {
    #[allow(dead_code)]
    iss: String,
    #[allow(dead_code)]
    iat: u64,
    #[allow(dead_code)]
    exp: u64,
    sub: Option<String>,
}

/// Authentication mode resolved at startup.
#[derive(Clone)]
pub enum AuthMode {
    /// JWT verification is enabled with the given decoding key and allowed users.
    Jwt {
        key: Arc<DecodingKey>,
        allowed_usernames: Vec<String>,
    },
    /// Authentication is explicitly disabled (insecure, for development only).
    Disabled,
}

/// Decode a PEM env var that may be raw PEM or base64-encoded PEM.
fn decode_pem_env(name: &str, value: &str) -> String {
    if value.starts_with("-----") {
        return value.to_string();
    }
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
        .unwrap_or_else(|e| panic!("{name} is not valid PEM or base64: {e}"));
    String::from_utf8(bytes).unwrap_or_else(|e| panic!("{name} base64 decoded to invalid UTF-8: {e}"))
}

/// Resolve the authentication mode from the API config section.
///
/// Call this once at startup before serving requests. Panics if the
/// configuration is invalid (JWT strategy but no public key).
pub fn resolve_auth_mode(
    api_config: &crate::server_config::ApiConfig,
    allowed_usernames: Vec<String>,
) -> AuthMode {
    use crate::server_config::ApiAuthenticationStrategy;

    match api_config.authentication_strategy {
        ApiAuthenticationStrategy::InsecureDisabled => {
            warn!("JWT authentication disabled");
            AuthMode::Disabled
        }
        ApiAuthenticationStrategy::Jwt => {
            let raw = std::env::var("ARC_JWT_PUBLIC_KEY").unwrap_or_else(|_| {
                panic!(
                    "ARC_JWT_PUBLIC_KEY is not set. Either provide an Ed25519 public key in PEM \
                     format (or base64-encoded PEM) or set authentication_strategy = \
                     \"insecure_disabled\" in ~/.arc/arc.toml to allow unauthenticated access \
                     (development only)."
                )
            });
            let pem = decode_pem_env("ARC_JWT_PUBLIC_KEY", &raw);
            let key = DecodingKey::from_ed_pem(pem.as_bytes())
                .expect("ARC_JWT_PUBLIC_KEY contains an invalid Ed25519 PEM public key");
            AuthMode::Jwt {
                key: Arc::new(key),
                allowed_usernames,
            }
        }
    }
}

/// Axum extractor that enforces JWT authentication on a route.
///
/// Add this as a parameter to any handler to require a valid JWT.
/// The `AuthMode` must be added to the router as an Extension.
/// When auth is disabled, the extractor accepts all requests.
pub struct AuthenticatedService;

impl<S: Send + Sync> FromRequestParts<S> for AuthenticatedService {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_mode = parts
            .extensions
            .get::<AuthMode>()
            .expect("AuthMode extension must be added to the router");

        let (key, allowed_usernames) = match auth_mode {
            AuthMode::Disabled => return Ok(AuthenticatedService),
            AuthMode::Jwt {
                key,
                allowed_usernames,
            } => (key, allowed_usernames),
        };

        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_required_spec_claims(&["iss", "iat", "exp"]);
        validation.set_issuer(&["arc-web"]);

        let token_data = jsonwebtoken::decode::<Claims>(token, key, &validation)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        // Fail closed: if no usernames are allowed, reject all requests
        if allowed_usernames.is_empty() {
            return Err(StatusCode::FORBIDDEN);
        }

        // Extract GitHub username from sub claim URL (last path segment)
        let username = token_data
            .claims
            .sub
            .as_deref()
            .and_then(|s| s.rsplit('/').next())
            .ok_or(StatusCode::FORBIDDEN)?;

        if !allowed_usernames.iter().any(|u| u == username) {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(AuthenticatedService)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn protected_handler(_auth: AuthenticatedService) -> impl IntoResponse {
        "ok"
    }

    fn test_router(mode: AuthMode) -> Router {
        Router::new()
            .route("/test", get(protected_handler))
            .layer(axum::Extension(mode))
    }

    fn generate_test_keypair() -> (jsonwebtoken::EncodingKey, DecodingKey) {
        let output = std::process::Command::new("openssl")
            .args(["genpkey", "-algorithm", "Ed25519"])
            .output()
            .expect("openssl must be available for tests");
        let private_pem = output.stdout;

        let output = std::process::Command::new("openssl")
            .args(["pkey", "-pubout"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(&private_pem).unwrap();
                child.wait_with_output()
            })
            .expect("openssl pkey failed");
        let public_pem = output.stdout;

        let encoding =
            jsonwebtoken::EncodingKey::from_ed_pem(&private_pem).expect("invalid private key");
        let decoding = DecodingKey::from_ed_pem(&public_pem).expect("invalid public key");
        (encoding, decoding)
    }

    fn sign_token(
        key: &jsonwebtoken::EncodingKey,
        iss: &str,
        exp_secs: u64,
        sub: Option<&str>,
    ) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut claims = serde_json::json!({
            "iss": iss,
            "iat": now,
            "exp": now + exp_secs,
        });
        if let Some(sub) = sub {
            claims["sub"] = serde_json::Value::String(sub.to_string());
        }
        let header = jsonwebtoken::Header::new(Algorithm::EdDSA);
        jsonwebtoken::encode(&header, &claims, key).expect("failed to sign token")
    }

    fn jwt_mode(decoding: DecodingKey, allowed_usernames: Vec<&str>) -> AuthMode {
        AuthMode::Jwt {
            key: Arc::new(decoding),
            allowed_usernames: allowed_usernames.into_iter().map(String::from).collect(),
        }
    }

    #[tokio::test]
    async fn rejects_missing_auth_header() {
        let (_, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_invalid_token() {
        let (_, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let req = Request::builder()
            .uri("/test")
            .header("authorization", "Bearer invalid.token.here")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accepts_valid_token_with_matching_username() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let token = sign_token(
            &encoding,
            "arc-web",
            60,
            Some("https://github.com/brynary"),
        );

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_token_with_non_matching_username() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let token = sign_token(
            &encoding,
            "arc-web",
            60,
            Some("https://github.com/someone-else"),
        );

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_token_with_missing_sub() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let token = sign_token(&encoding, "arc-web", 60, None);

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_when_allowed_usernames_empty() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec![]));

        let token = sign_token(
            &encoding,
            "arc-web",
            60,
            Some("https://github.com/brynary"),
        );

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_wrong_issuer() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let token = sign_token(
            &encoding,
            "wrong-issuer",
            60,
            Some("https://github.com/brynary"),
        );

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let (encoding, decoding) = generate_test_keypair();
        let app = test_router(jwt_mode(decoding, vec!["brynary"]));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = serde_json::json!({
            "iss": "arc-web",
            "iat": now - 200,
            "exp": now - 100,
            "sub": "https://github.com/brynary",
        });
        let header = jsonwebtoken::Header::new(Algorithm::EdDSA);
        let token = jsonwebtoken::encode(&header, &claims, &encoding).unwrap();

        let req = Request::builder()
            .uri("/test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn resolve_auth_mode_insecure_disabled() {
        use crate::server_config::{ApiAuthenticationStrategy, ApiConfig};

        let config = ApiConfig {
            authentication_strategy: ApiAuthenticationStrategy::InsecureDisabled,
            ..ApiConfig::default()
        };
        assert!(matches!(
            resolve_auth_mode(&config, vec![]),
            AuthMode::Disabled
        ));
    }

    #[tokio::test]
    async fn disabled_mode_allows_all_requests() {
        let app = test_router(AuthMode::Disabled);

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
