use crate::common::config::OidcConfig;
use crate::common::error::ApiError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// OIDC Discovery Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub end_session_endpoint: Option<String>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub scopes_supported: Option<Vec<String>>,
    pub claims_supported: Option<Vec<String>>,
    pub code_challenge_methods_supported: Option<Vec<String>>,
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
    pub introspection_endpoint: Option<String>,
    pub revocation_endpoint: Option<String>,
}

/// OIDC JWKS (JSON Web Key Set)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcJwks {
    pub keys: Vec<OidcJwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcJwk {
    pub kty: String,
    pub kid: Option<String>,
    pub use_: Option<String>,
    #[serde(rename = "use")]
    pub use_alias: Option<String>,
    pub alg: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub x5c: Option<Vec<String>>,
    pub x5t: Option<String>,
}

/// OIDC Token Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scope: Option<String>,
}

/// OIDC User Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub preferred_username: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

/// OIDC Auth Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcAuthRequest {
    pub url: String,
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
    pub code_challenge: String,
}

/// OIDC User
#[derive(Debug, Clone)]
pub struct OidcUser {
    pub subject: String,
    pub localpart: String,
    pub displayname: Option<String>,
    pub email: Option<String>,
}

/// OIDC ID Token Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcIdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub at_hash: Option<String>,
    pub c_hash: Option<String>,
}

/// OIDC Session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcSession {
    pub session_id: String,
    pub user_id: String,
    pub subject: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_refreshed_at: Option<i64>,
}

impl OidcSession {
    pub fn new(
        user_id: String,
        subject: String,
        access_token: String,
        refresh_token: Option<String>,
        id_token: Option<String>,
        expires_in_seconds: i64,
    ) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id,
            subject,
            access_token,
            refresh_token,
            id_token,
            created_at: now,
            expires_at: now + (expires_in_seconds * 1000),
            last_refreshed_at: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp_millis() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    pub fn update_tokens(
        &mut self,
        access_token: String,
        refresh_token: Option<String>,
        expires_in_seconds: i64,
    ) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
        self.expires_at = Utc::now().timestamp_millis() + (expires_in_seconds * 1000);
        self.last_refreshed_at = Some(Utc::now().timestamp_millis());
    }
}

/// OIDC Logout Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcLogoutRequest {
    pub id_token_hint: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
}

/// OIDC Logout Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcLogoutResponse {
    pub success: bool,
    pub redirect_url: Option<String>,
}

/// OIDC Introspection Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcIntrospectionResponse {
    pub active: bool,
    pub sub: Option<String>,
    pub aud: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
}

/// OIDC Service
pub struct OidcService {
    config: Arc<OidcConfig>,
    http_client: reqwest::Client,
    discovery: Option<OidcDiscoveryDocument>,
    jwks: Option<OidcJwks>,
    sessions: Arc<RwLock<HashMap<String, OidcSession>>>,
    state_cache: Arc<RwLock<HashMap<String, OidcAuthRequest>>>,
}

impl OidcService {
    pub fn new(config: Arc<OidcConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http_client,
            discovery: None,
            jwks: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            state_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    pub async fn discover(&mut self) -> Result<OidcDiscoveryDocument, ApiError> {
        let cached = self.discovery.clone();
        if let Some(discovery) = cached {
            return Ok(discovery);
        }

        let discovery_url = format!("{}/.well-known/openid-configuration", self.config.issuer);
        
        debug!("Fetching OIDC discovery document from {}", discovery_url);

        let response = self.http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch discovery document: {}", e)))?;

        if !response.status().is_success() {
            return Err(ApiError::internal(format!("Discovery request failed: {}", response.status())));
        }

        let discovery: OidcDiscoveryDocument = response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse discovery document: {}", e)))?;

        info!(
            issuer = %discovery.issuer,
            auth_endpoint = %discovery.authorization_endpoint,
            "OIDC discovery completed"
        );

        self.discovery = Some(discovery.clone());
        Ok(discovery)
    }

    pub async fn fetch_jwks(&mut self) -> Result<OidcJwks, ApiError> {
        if let Some(ref jwks) = self.jwks {
            return Ok(jwks.clone());
        }

        let jwks_uri = self.config.jwks_uri.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.jwks_uri))
            .ok_or_else(|| ApiError::internal("JWKS URI not configured"))?;

        debug!("Fetching OIDC JWKS from {}", jwks_uri);

        let response = self.http_client
            .get(jwks_uri)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch JWKS: {}", e)))?;

        if !response.status().is_success() {
            return Err(ApiError::internal(format!("JWKS request failed: {}", response.status())));
        }

        let jwks: OidcJwks = response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse JWKS: {}", e)))?;

        info!(key_count = jwks.keys.len(), "OIDC JWKS fetched");

        self.jwks = Some(jwks.clone());
        Ok(jwks)
    }

    pub fn get_authorization_url(&self, state: &str, redirect_uri: &str) -> String {
        let scope = self.config.scopes.join(" ");
        
        let default_auth = format!("{}/authorize", self.config.issuer);
        let auth_endpoint = self.config.authorization_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.authorization_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_auth);

        let mut url = url::Url::parse(auth_endpoint).unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", &self.config.client_id);
            query.append_pair("response_type", "code");
            query.append_pair("scope", &scope);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("state", state);
        }

        url.to_string()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OidcTokenResponse, ApiError> {
        let default_token = format!("{}/token", self.config.issuer);
        let token_endpoint = self.config.token_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.token_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_token);

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
        ];

        let mut request = self.http_client.post(token_endpoint).form(&params);

        if let Some(ref secret) = self.config.client_secret {
            request = request.basic_auth(&self.config.client_id, Some(secret));
        }

        let response = request.send().await
            .map_err(|e| ApiError::internal(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::internal(format!("Token exchange failed: {}", body)));
        }

        response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse token response: {}", e)))
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<OidcUserInfo, ApiError> {
        let default_userinfo = format!("{}/userinfo", self.config.issuer);
        let userinfo_endpoint = self.config.userinfo_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.userinfo_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_userinfo);

        let response = self.http_client
            .get(userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("UserInfo request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ApiError::internal(format!("UserInfo request failed: {}", response.status())));
        }

        response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse UserInfo: {}", e)))
    }

    pub fn map_user(&self, user_info: &OidcUserInfo) -> OidcUser {
        let mapping = &self.config.attribute_mapping;

        let localpart = mapping.localpart.as_ref()
            .and_then(|attr| Self::get_attribute(user_info, attr))
            .unwrap_or(&user_info.sub);

        let displayname = mapping.displayname.as_ref()
            .and_then(|attr| Self::get_attribute(user_info, attr))
            .map(|s| s.to_string());

        let email = mapping.email.as_ref()
            .and_then(|attr| Self::get_attribute(user_info, attr))
            .map(|s| s.to_string());

        OidcUser {
            subject: user_info.sub.clone(),
            localpart: localpart.to_string(),
            displayname,
            email,
        }
    }

    fn get_attribute<'a>(user_info: &'a OidcUserInfo, attr: &str) -> Option<&'a str> {
        match attr {
            "sub" => Some(&user_info.sub),
            "name" => user_info.name.as_deref(),
            "given_name" => user_info.given_name.as_deref(),
            "family_name" => user_info.family_name.as_deref(),
            "preferred_username" => user_info.preferred_username.as_deref(),
            "email" => user_info.email.as_deref(),
            "picture" => user_info.picture.as_deref(),
            "locale" => user_info.locale.as_deref(),
            _ => None,
        }
    }

    pub fn generate_state() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect()
    }

    pub fn generate_code_verifier() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..64).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect()
    }

    pub fn generate_code_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    pub fn generate_nonce() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect()
    }

    pub fn create_auth_request(&self, redirect_uri: &str) -> OidcAuthRequest {
        let state = Self::generate_state();
        let nonce = Self::generate_nonce();
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        let scope = self.config.scopes.join(" ");
        
        let default_auth = format!("{}/authorize", self.config.issuer);
        let auth_endpoint = self.config.authorization_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.authorization_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_auth);

        let mut url = url::Url::parse(auth_endpoint).unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", &self.config.client_id);
            query.append_pair("response_type", "code");
            query.append_pair("scope", &scope);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("state", &state);
            query.append_pair("nonce", &nonce);
            query.append_pair("code_challenge", &code_challenge);
            query.append_pair("code_challenge_method", "S256");
        }

        OidcAuthRequest {
            url: url.to_string(),
            state,
            nonce,
            code_verifier,
            code_challenge,
        }
    }

    pub async fn exchange_code_with_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<OidcTokenResponse, ApiError> {
        let default_token = format!("{}/token", self.config.issuer);
        let token_endpoint = self.config.token_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.token_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_token);

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.config.client_id),
            ("code_verifier", code_verifier),
        ];

        let mut request = self.http_client.post(token_endpoint).form(&params);

        if let Some(ref secret) = self.config.client_secret {
            request = request.basic_auth(&self.config.client_id, Some(secret));
        }

        let response = request.send().await
            .map_err(|e| ApiError::internal(format!("Token exchange failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::internal(format!("Token exchange failed: {}", body)));
        }

        response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse token response: {}", e)))
    }

    pub fn decode_id_token_claims(&self, id_token: &str) -> Result<OidcIdTokenClaims, ApiError> {
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() != 3 {
            return Err(ApiError::bad_request("Invalid ID token format"));
        }

        let payload = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| ApiError::bad_request(format!("Failed to decode ID token: {}", e)))?;

        let claims: OidcIdTokenClaims = serde_json::from_slice(&payload)
            .map_err(|e| ApiError::bad_request(format!("Failed to parse ID token claims: {}", e)))?;

        if claims.iss != self.config.issuer {
            return Err(ApiError::bad_request("ID token issuer mismatch"));
        }

        if claims.aud != self.config.client_id {
            return Err(ApiError::bad_request("ID token audience mismatch"));
        }

        let now = chrono::Utc::now().timestamp();
        if claims.exp < now {
            return Err(ApiError::bad_request("ID token has expired"));
        }

        Ok(claims)
    }

    pub fn validate_nonce(&self, claims: &OidcIdTokenClaims, expected_nonce: &str) -> Result<(), ApiError> {
        if let Some(ref nonce) = claims.nonce {
            if nonce != expected_nonce {
                return Err(ApiError::bad_request("Nonce mismatch"));
            }
        }
        Ok(())
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OidcTokenResponse, ApiError> {
        let default_token = format!("{}/token", self.config.issuer);
        let token_endpoint = self.config.token_endpoint.as_ref()
            .or_else(|| self.discovery.as_ref().map(|d| &d.token_endpoint))
            .map(|s| s.as_str())
            .unwrap_or(&default_token);

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
        ];

        let mut request = self.http_client.post(token_endpoint).form(&params);

        if let Some(ref secret) = self.config.client_secret {
            request = request.basic_auth(&self.config.client_id, Some(secret));
        }

        let response = request.send().await
            .map_err(|e| ApiError::internal(format!("Token refresh failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::internal(format!("Token refresh failed: {}", body)));
        }

        response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse token response: {}", e)))
    }

    pub fn get_logout_url(&self, logout_request: &OidcLogoutRequest) -> Option<String> {
        let end_session_endpoint = self.config.jwks_uri.as_ref()
            .or_else(|| self.discovery.as_ref().and_then(|d| d.end_session_endpoint.as_ref()))?;

        let mut url = url::Url::parse(end_session_endpoint).ok()?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(ref id_token_hint) = logout_request.id_token_hint {
                query.append_pair("id_token_hint", id_token_hint);
            }
            if let Some(ref redirect_uri) = logout_request.post_logout_redirect_uri {
                query.append_pair("post_logout_redirect_uri", redirect_uri);
            }
            if let Some(ref state) = logout_request.state {
                query.append_pair("state", state);
            }
        }

        Some(url.to_string())
    }

    pub async fn logout(&self, session_id: &str) -> Result<OidcLogoutResponse, ApiError> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.remove(session_id) {
            info!(
                session_id = %session_id,
                user_id = %session.user_id,
                "OIDC session terminated"
            );

            if let Some(ref id_token) = session.id_token {
                let logout_request = OidcLogoutRequest {
                    id_token_hint: Some(id_token.clone()),
                    post_logout_redirect_uri: None,
                    state: None,
                };

                if let Some(logout_url) = self.get_logout_url(&logout_request) {
                    return Ok(OidcLogoutResponse {
                        success: true,
                        redirect_url: Some(logout_url),
                    });
                }
            }

            Ok(OidcLogoutResponse {
                success: true,
                redirect_url: None,
            })
        } else {
            Ok(OidcLogoutResponse {
                success: false,
                redirect_url: None,
            })
        }
    }

    pub async fn create_session(
        &self,
        user_id: String,
        subject: String,
        token_response: &OidcTokenResponse,
    ) -> OidcSession {
        let expires_in = token_response.expires_in.unwrap_or(3600);
        
        let session = OidcSession::new(
            user_id,
            subject,
            token_response.access_token.clone(),
            token_response.refresh_token.clone(),
            token_response.id_token.clone(),
            expires_in,
        );

        self.sessions.write().await.insert(session.session_id.clone(), session.clone());

        info!(
            session_id = %session.session_id,
            expires_in = expires_in,
            "OIDC session created"
        );

        session
    }

    pub async fn get_session(&self, session_id: &str) -> Option<OidcSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn validate_session(&self, session_id: &str) -> bool {
        self.sessions
            .read()
            .await
            .get(session_id)
            .map(|s| s.is_valid())
            .unwrap_or(false)
    }

    pub async fn refresh_session(&self, session_id: &str) -> Result<OidcSession, ApiError> {
        let mut sessions = self.sessions.write().await;
        
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ApiError::not_found("Session not found"))?;

        let refresh_token = session.refresh_token.clone()
            .ok_or_else(|| ApiError::bad_request("No refresh token available"))?;

        let token_response = self.refresh_token(&refresh_token).await?;

        session.update_tokens(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in.unwrap_or(3600),
        );

        info!(session_id = %session_id, "OIDC session refreshed");

        Ok(session.clone())
    }

    pub async fn introspect_token(&self, token: &str) -> Result<OidcIntrospectionResponse, ApiError> {
        let introspection_endpoint = self.discovery.as_ref()
            .and_then(|d| d.introspection_endpoint.as_ref());

        let Some(endpoint) = introspection_endpoint else {
            return Ok(OidcIntrospectionResponse {
                active: false,
                sub: None,
                aud: None,
                exp: None,
                iat: None,
                scope: None,
                token_type: None,
            });
        };

        let params = [("token", token)];

        let mut request = self.http_client.post(endpoint).form(&params);

        if let Some(ref secret) = self.config.client_secret {
            request = request.basic_auth(&self.config.client_id, Some(secret));
        }

        let response = request.send().await
            .map_err(|e| ApiError::internal(format!("Token introspection failed: {}", e)))?;

        if !response.status().is_success() {
            return Ok(OidcIntrospectionResponse {
                active: false,
                sub: None,
                aud: None,
                exp: None,
                iat: None,
                scope: None,
                token_type: None,
            });
        }

        response.json().await
            .map_err(|e| ApiError::internal(format!("Failed to parse introspection response: {}", e)))
    }

    pub async fn revoke_token(&self, token: &str, token_type_hint: Option<&str>) -> Result<(), ApiError> {
        let revocation_endpoint = self.discovery.as_ref()
            .and_then(|d| d.revocation_endpoint.as_ref());

        let Some(endpoint) = revocation_endpoint else {
            debug!("Token revocation not supported by provider");
            return Ok(());
        };

        let mut params = vec![("token", token)];
        if let Some(hint) = token_type_hint {
            params.push(("token_type_hint", hint));
        }

        let mut request = self.http_client.post(endpoint).form(&params);

        if let Some(ref secret) = self.config.client_secret {
            request = request.basic_auth(&self.config.client_id, Some(secret));
        }

        let response = request.send().await
            .map_err(|e| ApiError::internal(format!("Token revocation failed: {}", e)))?;

        if response.status().is_success() {
            info!("Token revoked successfully");
        } else {
            debug!("Token revocation returned non-success status");
        }

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        
        sessions.retain(|_, s| s.is_valid());
        
        let removed = before - sessions.len();
        if removed > 0 {
            debug!(count = removed, "Expired OIDC sessions cleaned up");
        }
        removed
    }

    pub async fn store_auth_request(&self, state: &str, auth_request: OidcAuthRequest) {
        self.state_cache.write().await.insert(state.to_string(), auth_request);
    }

    pub async fn get_auth_request(&self, state: &str) -> Option<OidcAuthRequest> {
        self.state_cache.write().await.remove(state)
    }

    pub fn get_config(&self) -> &OidcConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::config::OidcAttributeMapping;

    fn create_test_config() -> OidcConfig {
        OidcConfig {
            enabled: true,
            issuer: "https://accounts.example.com".to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: Some("test-client-secret".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            attribute_mapping: OidcAttributeMapping {
                localpart: Some("preferred_username".to_string()),
                displayname: Some("name".to_string()),
                email: Some("email".to_string()),
            },
            callback_url: Some("https://matrix.example.com/_matrix/client/r0/login/sso/redirect".to_string()),
            allow_existing_users: true,
            block_unknown_users: false,
            authorization_endpoint: None,
            token_endpoint: None,
            userinfo_endpoint: None,
            jwks_uri: None,
            timeout: 10,
        }
    }

    fn create_test_service() -> OidcService {
        let config = Arc::new(create_test_config());
        OidcService::new(config)
    }

    #[test]
    fn test_oidc_config_enabled() {
        let config = create_test_config();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_oidc_config_disabled() {
        let config = OidcConfig::default();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_service_enabled() {
        let service = create_test_service();
        assert!(service.is_enabled());
    }

    #[test]
    fn test_generate_state() {
        let state = OidcService::generate_state();
        assert_eq!(state.len(), 32);
        assert!(state.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_get_authorization_url() {
        let service = create_test_service();
        let url = service.get_authorization_url("test-state", "https://matrix.example.com/callback");
        
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("scope="));
    }

    #[test]
    fn test_map_user() {
        let service = create_test_service();
        let user_info = OidcUserInfo {
            sub: "user123".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            preferred_username: Some("testuser".to_string()),
            email: Some("test@example.com".to_string()),
            email_verified: Some(true),
            picture: Some("https://example.com/avatar.png".to_string()),
            locale: Some("en".to_string()),
        };

        let user = service.map_user(&user_info);
        
        assert_eq!(user.subject, "user123");
        assert_eq!(user.localpart, "testuser");
        assert_eq!(user.displayname, Some("Test User".to_string()));
        assert_eq!(user.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_map_user_default_localpart() {
        let mut config = create_test_config();
        config.attribute_mapping.localpart = None;
        let service = OidcService::new(Arc::new(config));
        
        let user_info = OidcUserInfo {
            sub: "user123".to_string(),
            name: None,
            given_name: None,
            family_name: None,
            preferred_username: None,
            email: None,
            email_verified: None,
            picture: None,
            locale: None,
        };

        let user = service.map_user(&user_info);
        assert_eq!(user.localpart, "user123");
    }

    #[test]
    fn test_generate_code_verifier() {
        let verifier = OidcService::generate_code_verifier();
        assert_eq!(verifier.len(), 64);
        assert!(verifier.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_code_challenge() {
        let verifier = "test_verifier_1234567890";
        let challenge = OidcService::generate_code_challenge(verifier);
        
        assert!(!challenge.is_empty());
        assert_ne!(challenge, verifier);
    }

    #[test]
    fn test_generate_nonce() {
        let nonce = OidcService::generate_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_create_auth_request() {
        let service = create_test_service();
        let auth_request = service.create_auth_request("https://matrix.example.com/callback");
        
        assert!(auth_request.url.contains("client_id=test-client-id"));
        assert!(auth_request.url.contains("response_type=code"));
        assert!(auth_request.url.contains("code_challenge="));
        assert!(auth_request.url.contains("code_challenge_method=S256"));
        assert_eq!(auth_request.state.len(), 32);
        assert_eq!(auth_request.nonce.len(), 32);
        assert_eq!(auth_request.code_verifier.len(), 64);
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test_verifier";
        let challenge1 = OidcService::generate_code_challenge(verifier);
        let challenge2 = OidcService::generate_code_challenge(verifier);
        
        assert_eq!(challenge1, challenge2);
    }
}
