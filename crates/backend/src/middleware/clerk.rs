use crate::secret_str::SecretStr;
use axum::body::Body;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use base64ct::Encoding;
use clerk_client::apis::Api;
use clerk_client::apis::configuration::Configuration;
use clerk_client::models::JwksKeysInner;
use clerk_client::models::jwks_rsa_public_key::Kty;
use clerk_client::models::session::Status;
use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use jsonwebtoken::decode;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;

#[cfg(feature = "serde")]
fn deserialize_u64_from_str_or_num<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    match value {
        Value::Number(num) => num
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("invalid u64 number")),
        Value::String(s) => s
            .parse::<u64>()
            .map_err(|_| serde::de::Error::custom("invalid u64 string")),
        _ => Err(serde::de::Error::custom(
            "expected number or string for u64",
        )),
    }
}

#[derive(Clone)]
pub struct CachedKey {
    pub key: DecodingKey,
    pub fetched_at: Instant,
}

#[derive(Clone)]
pub struct ClerkState {
    pub clerk_config: Configuration,
    pub jwks_cache: Arc<Mutex<HashMap<String, CachedKey>>>,
    pub api_client: Arc<clerk_client::apis::ApiClient>,
    pub issuer: String,
    pub audience: String,
    pub jwks_cache_ttl: Duration,
    pub require_azp: bool,
    pub jwt_leeway_seconds: u64,
}

impl ClerkState {
    pub fn configured(
        SecretStr(bearer_access_token): SecretStr,
        issuer: String,
        audience: String,
        require_azp: bool,
        jwt_leeway_seconds: u64,
        jwks_cache_ttl: Duration,
    ) -> Self {
        let mut clerk_config = Configuration::default();
        clerk_config.bearer_access_token = Some(bearer_access_token);

        let api_client = Arc::new(clerk_client::apis::ApiClient::new(Arc::new(
            clerk_config.clone(),
        )));

        Self {
            jwks_cache: Arc::new(Mutex::new(HashMap::new())),
            clerk_config,
            api_client,
            issuer,
            audience,
            jwks_cache_ttl,
            require_azp,
            jwt_leeway_seconds,
        }
    }

    /// Extract JWT token from Authorization header
    pub fn extract_token_from_header(
        &self,
        request: &Request<Body>,
    ) -> Result<String, TokenExtractionError> {
        let auth_header = request
            .headers()
            .get(AUTHORIZATION)
            .ok_or(TokenExtractionError::MissingToken)?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| TokenExtractionError::InvalidTokenFormat)?;

        const BEARER_: &'static str = "Bearer ";

        if !auth_str.starts_with(BEARER_) {
            return Err(TokenExtractionError::InvalidTokenFormat);
        }

        let token = auth_str[BEARER_.len()..].trim();
        if token.is_empty() {
            return Err(TokenExtractionError::InvalidTokenFormat);
        }

        Ok(token.to_string())
    }

    /// Get or fetch JWKS keys for JWT verification
    pub async fn get_or_fetch_jwks(&self, kid: &str) -> Result<DecodingKey, JwksError> {
        // Check cache first with TTL validation
        let mut cache = self.jwks_cache.lock().await;
        if let Some(cached_key) = cache.get(kid) {
            if cached_key.fetched_at.elapsed() <= self.jwks_cache_ttl {
                tracing::debug!("Using cached JWKS key for kid: {}", &kid[..8]);
                return Ok(cached_key.key.clone());
            } else {
                tracing::debug!("Cached JWKS key expired for kid: {}", &kid[..8]);
            }
        }

        // Fetch from Clerk API
        tracing::info!("Fetching JWKS from Clerk API");
        let jwks = self.api_client.jwks_api().get_jwks().await.map_err(|e| {
            tracing::error!("Failed to fetch JWKS: {:?}", e);
            JwksError::JwksFetchError
        })?;

        // Parse and cache keys
        let now = Instant::now();
        let mut found_key = None;

        // Clean expired entries, but keep them as fallback if fetch fails
        let mut expired_keys = Vec::new();
        cache.retain(|kid_cached, cached_key| {
            if cached_key.fetched_at.elapsed() > self.jwks_cache_ttl {
                expired_keys.push((kid_cached.clone(), cached_key.key.clone()));
                false
            } else {
                true
            }
        });

        if let Some(keys) = jwks.keys {
            for key in keys {
                if let Ok((key_id, decoding_key)) = self.parse_jwks_key(&key) {
                    cache.insert(
                        key_id.clone(),
                        CachedKey {
                            key: decoding_key.clone(),
                            fetched_at: now,
                        },
                    );
                    if key_id == kid {
                        found_key = Some(decoding_key);
                    }
                } else {
                    // parsing errors for particular keys are ignored here; parse_jwks_key returns JwksError
                    // we continue processing other keys
                    continue;
                }
            }
        }

        // If we found the key, return it
        if let Some(key) = found_key {
            return Ok(key);
        }

        // If we failed to find the requested key, try using expired keys as fallback
        for (expired_kid, expired_key) in expired_keys {
            if expired_kid == kid {
                tracing::warn!("Using expired JWKS key as fallback for kid: {}", &kid[..8]);
                return Ok(expired_key);
            }
        }

        Err(JwksError::KeyNotFound)
    }

    /// Parse JWKS key into DecodingKey
    fn parse_jwks_key(&self, key: &JwksKeysInner) -> Result<(String, DecodingKey), JwksError> {
        use Kty::Rsa;

        match key {
            JwksKeysInner::JwksRsaPublicKey(rsa_key) => {
                if rsa_key.kty == Rsa {
                    // Validate kid is present and non-empty
                    let kid_str = rsa_key.kid.trim();
                    if kid_str.is_empty() {
                        return Err(JwksError::KeyParseError);
                    }

                    // Create decoding key with components
                    let decoding_key = DecodingKey::from_rsa_components(&rsa_key.n, &rsa_key.e)
                        .map_err(|_| JwksError::KeyParseError)?;

                    Ok((kid_str.to_string(), decoding_key))
                } else {
                    Err(JwksError::UnsupportedKeyType)
                }
            }
            JwksKeysInner::JwksEcdsaPublicKey(_) => {
                // ECDSA keys would require more complex handling
                Err(JwksError::UnsupportedKeyType)
            }
            JwksKeysInner::JwksEd25519PublicKey(_) => {
                // Ed25519 keys would require different handling
                Err(JwksError::UnsupportedKeyType)
            }
            // Private keys shouldn't be in JWKS for verification
            JwksKeysInner::JwksRsaPrivateKey(_)
            | JwksKeysInner::JwksEcdsaPrivateKey(_)
            | JwksKeysInner::JwksEd25519PrivateKey(_) => Err(JwksError::UnsupportedKeyType),
            JwksKeysInner::JwksSymmetricKey(_) => Err(JwksError::UnsupportedKeyType),
        }
    }

    /// Verify JWT token and return user claims
    pub async fn verify_jwt(
        &self,
        token: &str,
    ) -> Result<TokenData<ClerkUser>, JwtVerificationError> {
        // Extract kid from token header
        let kid = self.extract_kid_from_token(token)?;

        // Get decoding key
        let decoding_key = self.get_or_fetch_jwks(&kid).await?;

        // Configure validation with leeway for clock skew
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = self.jwt_leeway_seconds; // Add leeway for clock skew

        // Decode and verify token
        let token_data = decode::<ClerkUser>(token, &decoding_key, &validation).map_err(|e| {
            tracing::warn!("JWT verification failed: {:?}", e);
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    JwtVerificationError::TokenExpired(e)
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    JwtVerificationError::InvalidSignature
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => JwtVerificationError::InvalidToken,
                _ => JwtVerificationError::VerificationError,
            }
        })?;

        // clerk issues azp not aud in jwts so this is fine
        if token_data.claims.azp != self.audience {
            tracing::warn!(
                "Invalid azp: {} expected: {}",
                &token_data.claims.azp[..8],
                &self.audience[..8]
            );
            return Err(JwtVerificationError::InvalidToken);
        }

        // Mask user_id in logs for security
        let masked_user_id = if token_data.claims.sub.len() > 8 {
            format!(
                "{}...{}",
                &token_data.claims.sub[..4],
                &token_data.claims.sub[token_data.claims.sub.len() - 4..]
            )
        } else {
            "[masked]".to_owned()
        };
        tracing::debug!("JWT verified successfully for user: {}", masked_user_id);

        Ok(token_data)
    }

    /// Extract key ID (kid) from JWT token header
    fn extract_kid_from_token(&self, token: &str) -> Result<String, KidExtractionError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(KidExtractionError::InvalidTokenFormat);
        }

        let header_data = base64ct::Base64::decode_vec(&parts[0])
            .map_err(|_| KidExtractionError::Base64NormalizationError)?;

        let header: serde_json::Value = serde_json::from_slice(&header_data)
            .map_err(|_| KidExtractionError::InvalidTokenFormat)?;

        let kid = header
            .get("kid")
            .and_then(|v| v.as_str())
            .ok_or(KidExtractionError::KeyIdMissing)?;

        Ok(kid.to_string())
    }

    /// Verify session status with Clerk API
    /// This can be called after JWT verification for additional security
    pub async fn verify_session_status(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<(), SessionError> {
        // Mask sensitive identifiers in logs
        let masked_user_id = if user_id.len() > 8 {
            format!("{}...{}", &user_id[..4], &user_id[user_id.len() - 4..])
        } else {
            "[masked]".to_owned()
        };
        let masked_session_id = if session_id.len() > 8 {
            format!(
                "{}...{}",
                &session_id[..4],
                &session_id[session_id.len() - 4..]
            )
        } else {
            "[masked]".to_owned()
        };

        tracing::debug!(
            "Session verification requested for user: {}, session: {}",
            masked_user_id,
            masked_session_id
        );

        // Verify session using Clerk's sessions API
        let session = self
            .api_client
            .sessions_api()
            .get_session(session_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify session: {:?}", e);
                SessionError::SessionRevocationError
            })?;

        // Validate that the session belongs to the user and is active
        if session.user_id != user_id {
            tracing::warn!(
                "Session {} does not belong to user {}",
                masked_session_id,
                masked_user_id
            );
            return Err(SessionError::SessionRevocationError);
        }

        // Check if session is active (not revoked or expired)
        if session.status != Status::Active {
            tracing::warn!(
                "Session {} is not active (status: {:?})",
                masked_session_id,
                session.status
            );
            return Err(SessionError::SessionRevocationError);
        }

        tracing::info!(
            "Session verification successful for user: {}, session: {}",
            masked_user_id,
            masked_session_id
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClerkUser {
    pub sub: String,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub exp: u64,
    #[serde(deserialize_with = "deserialize_u64_from_str_or_num")]
    pub iat: u64,
    pub azp: String,
    pub sid: String,
    #[serde(default, deserialize_with = "deserialize_u64_from_str_or_num")]
    pub nbf: u64,
}

#[derive(Debug, Clone)]
pub struct Clerk {
    pub user: ClerkUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClerkUserId(pub String);

impl Display for ClerkUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Clerk {
    pub fn user_id(&self) -> ClerkUserId {
        ClerkUserId(self.user.sub.to_owned())
    }
}

/// Token extraction errors
#[derive(Debug, thiserror::Error)]
pub enum TokenExtractionError {
    #[error("Missing authentication token")]
    MissingToken,
    #[error("Invalid token format")]
    InvalidTokenFormat,
}

/// Kid/header extraction errors
#[derive(Debug, thiserror::Error)]
pub enum KidExtractionError {
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Key ID missing from token")]
    KeyIdMissing,
    #[error("Base64 normalization error")]
    Base64NormalizationError,
}

/// JWKS-related errors
#[derive(Debug, thiserror::Error)]
pub enum JwksError {
    #[error("Failed to fetch JWKS")]
    JwksFetchError,
    #[error("Key not found")]
    KeyNotFound,
    #[error("Key parse error")]
    KeyParseError,
    #[error("Unsupported key type")]
    UnsupportedKeyType,
}

/// JWT verification errors
#[derive(Debug, thiserror::Error)]
pub enum JwtVerificationError {
    #[error("Token expired")]
    TokenExpired(#[from] jsonwebtoken::errors::Error),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Verification error")]
    VerificationError,
}

/// Session verification errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session revocation check failed")]
    SessionRevocationError,
}

/// Trait to convert error types into an HTTP response (status + message)
pub trait ToAuthResponse {
    fn status_code(&self) -> StatusCode;
    fn message(&self) -> String;
}

impl ToAuthResponse for TokenExtractionError {
    fn status_code(&self) -> StatusCode {
        match self {
            TokenExtractionError::MissingToken => StatusCode::UNAUTHORIZED,
            TokenExtractionError::InvalidTokenFormat => StatusCode::UNAUTHORIZED,
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
}

impl ToAuthResponse for KidExtractionError {
    fn status_code(&self) -> StatusCode {
        match self {
            KidExtractionError::InvalidTokenFormat => StatusCode::UNAUTHORIZED,
            KidExtractionError::KeyIdMissing => StatusCode::UNAUTHORIZED,
            KidExtractionError::Base64NormalizationError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
}

impl ToAuthResponse for JwksError {
    fn status_code(&self) -> StatusCode {
        match self {
            JwksError::JwksFetchError => StatusCode::INTERNAL_SERVER_ERROR,
            JwksError::KeyNotFound => StatusCode::UNAUTHORIZED,
            JwksError::KeyParseError => StatusCode::INTERNAL_SERVER_ERROR,
            JwksError::UnsupportedKeyType => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
}

impl ToAuthResponse for JwtVerificationError {
    fn status_code(&self) -> StatusCode {
        match self {
            JwtVerificationError::TokenExpired(_) => StatusCode::UNAUTHORIZED,
            JwtVerificationError::InvalidSignature => StatusCode::UNAUTHORIZED,
            JwtVerificationError::InvalidToken => StatusCode::UNAUTHORIZED,
            JwtVerificationError::VerificationError => StatusCode::UNAUTHORIZED,
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
}

impl ToAuthResponse for SessionError {
    fn status_code(&self) -> StatusCode {
        match self {
            SessionError::SessionRevocationError => StatusCode::UNAUTHORIZED,
        }
    }
    fn message(&self) -> String {
        self.to_string()
    }
}

// Conversions between error types to allow `?` propagation across layers
impl From<KidExtractionError> for JwtVerificationError {
    fn from(_: KidExtractionError) -> Self {
        // Any kid/header extraction failure is considered a token format/verification issue
        JwtVerificationError::InvalidToken
    }
}

impl From<JwksError> for JwtVerificationError {
    fn from(e: JwksError) -> Self {
        match e {
            JwksError::KeyNotFound => JwtVerificationError::InvalidToken,
            JwksError::JwksFetchError => JwtVerificationError::VerificationError,
            JwksError::KeyParseError => JwtVerificationError::VerificationError,
            JwksError::UnsupportedKeyType => JwtVerificationError::VerificationError,
        }
    }
}

/// Create error response from any error implementing ToAuthResponse
fn create_error_response_from<E: ToAuthResponse>(err: E) -> Response {
    let error_message = err.message();
    let status_code = err.status_code();

    let body = serde_json::json!({
        "error": error_message,
        "status": status_code.as_u16(),
    });

    Response::builder()
        .status(status_code)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_owned()),
        ))
        .unwrap_or_else(|_| Response::new(Body::from("Internal Server Error")))
}

pub async fn validate_clerk_session(
    State(state): State<ClerkState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let token = match state.extract_token_from_header(&request) {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!("Authentication failed: {}", err);
            return create_error_response_from(err);
        }
    };

    let token_data = match state.verify_jwt(&token).await {
        Ok(token_data) => token_data,
        Err(err) => {
            tracing::warn!("JWT verification failed: {}", err);
            return create_error_response_from(err);
        }
    };

    let session_id = &*token_data.claims.sid;

    if let Err(err) = state
        .verify_session_status(&token_data.claims.sub, session_id)
        .await
    {
        return create_error_response_from(err);
    }

    let clerk = Clerk {
        user: token_data.claims,
    };

    request.extensions_mut().insert(clerk);

    next.run(request).await
}
