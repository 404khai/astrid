//! App-owned ChatGPT authentication. Never opens Codex's credential store.
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const ISSUER: &str = "https://auth.openai.com";
const RESOURCE: &str = "https://api.openai.com/v1";
const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/api/accounts/oauth/token";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("authentication transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("{0}")]
    Invalid(String),
}

/// Intentionally has no Debug or Serialize implementation.
pub struct BearerToken(String);
impl BearerToken {
    pub fn new(value: String) -> Result<Self, AuthError> {
        if value.trim().is_empty() {
            return Err(AuthError::Invalid("empty bearer credential".into()));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[async_trait]
pub trait Authentication: Send + Sync {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError>;
}

pub fn default_directory() -> Result<PathBuf, AuthError> {
    Ok(PathBuf::from(
        std::env::var_os("HOME").ok_or_else(|| AuthError::Invalid("HOME is not set".into()))?,
    )
    .join(".config/astrid"))
}

/// One selected account in Phase 0; credential lifetime is separate from runs.
pub struct ChatGptAuth {
    directory: PathBuf,
    client: reqwest::Client,
}

impl ChatGptAuth {
    pub fn new(directory: PathBuf) -> Result<Self, AuthError> {
        Ok(Self {
            directory,
            client: client()?,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    issuer: String,
    client_id: String,
    subject: String,
    email: Option<String>,
    ext_agent_host_id: String,
    id_token: String,
    access_token: String,
    refresh_token: String,
    scopes: Vec<String>,
    expires_at: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Identity {
    sub: String,
    nonce: Option<String>,
    email: Option<String>,
}

fn client() -> Result<reqwest::Client, AuthError> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}
fn now() -> Result<u64, AuthError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::Invalid("system clock precedes Unix epoch".into()))?
        .as_secs())
}
fn random() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn protected_directory(path: &Path) -> Result<(), AuthError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(AuthError::Invalid(
                "authentication directory must be a real directory".into(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir_all(path)?,
        Err(error) => return Err(error.into()),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), AuthError> {
    let parent = path
        .parent()
        .ok_or_else(|| AuthError::Invalid("invalid credential path".into()))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer(&mut file, value)
        .map_err(|_| AuthError::Invalid("could not serialize credentials".into()))?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn load(path: &Path) -> Result<Option<Credentials>, AuthError> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if file.metadata()?.permissions().mode() & 0o077 != 0 {
        return Err(AuthError::Invalid(
            "credential file must have owner-only permissions (0600)".into(),
        ));
    }
    let credentials: Credentials = serde_json::from_reader(file).map_err(|_| {
        AuthError::Invalid("invalid Astrid credentials; run astrid login again".into())
    })?;
    if credentials.issuer != ISSUER
        || credentials.client_id.is_empty()
        || credentials.client_id == "dynamic_agent_client"
        || credentials.subject.is_empty()
    {
        return Err(AuthError::Invalid(
            "invalid Astrid account registration".into(),
        ));
    }
    require_plan(&credentials.scopes)?;
    Ok(Some(credentials))
}

async fn lock(directory: &Path) -> Result<File, AuthError> {
    let path = directory.join("auth.lock");
    tokio::task::spawn_blocking(move || {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        file.lock()?;
        Ok::<_, io::Error>(file)
    })
    .await
    .map_err(|_| AuthError::Invalid("credential lock failed".into()))?
    .map_err(AuthError::Io)
}

fn require_plan(scopes: &[String]) -> Result<(), AuthError> {
    for required in ["resource.invoke", "chatgpt.tokens.use.direct"] {
        if !scopes.iter().any(|scope| scope == required) {
            return Err(AuthError::Invalid(format!(
                "ChatGPT plan permission {required} was not granted; run astrid login"
            )));
        }
    }
    Ok(())
}

async fn token_request(
    client: &reqwest::Client,
    form: &[(&str, &str)],
) -> Result<TokenResponse, AuthError> {
    let response = client.post(TOKEN_ENDPOINT).form(form).send().await?;
    if !response.status().is_success() {
        // Do not echo credential-containing request URLs or token endpoint bodies.
        return Err(AuthError::Invalid(format!(
            "OpenAI token exchange failed (HTTP {}); sign in again",
            response.status().as_u16()
        )));
    }
    let tokens: TokenResponse = response.json().await?;
    if !tokens.token_type.eq_ignore_ascii_case("bearer")
        || tokens.access_token.trim().is_empty()
        || tokens.expires_in == 0
    {
        return Err(AuthError::Invalid("invalid token response".into()));
    }
    Ok(tokens)
}

fn verify_identity(
    token: &str,
    client_id: &str,
    nonce: Option<&str>,
    jwks: &JwkSet,
) -> Result<Identity, AuthError> {
    let header =
        decode_header(token).map_err(|_| AuthError::Invalid("invalid ID token header".into()))?;
    if header.alg != Algorithm::RS256 {
        return Err(AuthError::Invalid(
            "unsupported ID token signature algorithm".into(),
        ));
    }
    let kid = header
        .kid
        .ok_or_else(|| AuthError::Invalid("ID token has no key ID".into()))?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| AuthError::Invalid("ID token signing key not found".into()))?;
    let key = DecodingKey::from_jwk(jwk)
        .map_err(|_| AuthError::Invalid("invalid OpenAI signing key".into()))?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    validation.set_required_spec_claims(&["exp", "iat", "sub", "iss", "aud"]);
    validation.leeway = 5;
    let identity = decode::<Identity>(token, &key, &validation)
        .map_err(|_| AuthError::Invalid("ID token signature or claims validation failed".into()))?
        .claims;
    if identity.sub.is_empty()
        || nonce.is_some_and(|expected| identity.nonce.as_deref() != Some(expected))
    {
        return Err(AuthError::Invalid(
            "ID token identity or nonce did not match".into(),
        ));
    }
    Ok(identity)
}

async fn jwks(client: &reqwest::Client) -> Result<JwkSet, AuthError> {
    Ok(client
        .get("https://auth.openai.com/.well-known/jwks.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

#[async_trait]
impl Authentication for ChatGptAuth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        if !self.directory.exists() {
            return Err(AuthError::Invalid("not signed in; run astrid login".into()));
        }
        protected_directory(&self.directory)?;
        let _lock = lock(&self.directory).await?;
        let path = self.directory.join("credentials.json");
        let mut saved = load(&path)?
            .ok_or_else(|| AuthError::Invalid("not signed in; run astrid login".into()))?;
        if saved.expires_at <= now()?.saturating_add(60) {
            let tokens = token_request(
                &self.client,
                &[
                    ("grant_type", "refresh_token"),
                    ("client_id", &saved.client_id),
                    ("refresh_token", &saved.refresh_token),
                    ("resource", RESOURCE),
                ],
            )
            .await?;
            if let Some(id_token) = tokens.id_token {
                let identity = verify_identity(
                    &id_token,
                    &saved.client_id,
                    None,
                    &jwks(&self.client).await?,
                )?;
                if identity.sub != saved.subject {
                    return Err(AuthError::Invalid(
                        "refresh returned a different account".into(),
                    ));
                }
                saved.id_token = id_token;
            }
            if let Some(scopes) = tokens.scope {
                saved.scopes = scopes.split_whitespace().map(str::to_owned).collect();
            }
            require_plan(&saved.scopes)?;
            saved.access_token = tokens.access_token;
            if let Some(refresh) = tokens.refresh_token {
                saved.refresh_token = refresh;
            }
            saved.expires_at = now()?
                .checked_add(tokens.expires_in)
                .ok_or_else(|| AuthError::Invalid("invalid token expiry".into()))?;
            save(&path, &saved)?;
        }
        BearerToken::new(saved.access_token)
    }
}

struct LoginAttempt {
    state: String,
    nonce: String,
    verifier: String,
    redirect_uri: String,
    client_id: Option<String>,
}
impl LoginAttempt {
    fn url(&self, host: &str) -> Result<reqwest::Url, AuthError> {
        let mut url = reqwest::Url::parse("https://auth.openai.com/api/accounts/authorize")
            .map_err(|_| AuthError::Invalid("invalid authorization URL".into()))?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()));
        let mut query = url.query_pairs_mut();
        query.extend_pairs([
            (
                "client_id",
                self.client_id.as_deref().unwrap_or("dynamic_agent_client"),
            ),
            ("ext_agent_host_id", host),
            ("response_type", "code"),
            ("redirect_uri", &self.redirect_uri),
            ("scope", SCOPES),
            ("resource", RESOURCE),
            ("state", &self.state),
            ("nonce", &self.nonce),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
        ]);
        if self.client_id.is_none() {
            query.append_pair("agent_name_hint", "Astrid");
        }
        drop(query);
        Ok(url)
    }

    fn callback(&self, target: &str) -> Result<(String, String), AuthError> {
        let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
            .map_err(|_| AuthError::Invalid("invalid callback URL".into()))?;
        if url.path() != "/auth/callback" {
            return Err(AuthError::Invalid("invalid callback path".into()));
        }
        let mut params = HashMap::new();
        for (key, value) in url.query_pairs() {
            if params
                .insert(key.into_owned(), value.into_owned())
                .is_some()
            {
                return Err(AuthError::Invalid("duplicated callback parameter".into()));
            }
        }
        if params.get("state") != Some(&self.state) {
            return Err(AuthError::Invalid("callback state did not match".into()));
        }
        if params.contains_key("error") {
            return Err(AuthError::Invalid(
                "ChatGPT authorization was denied or unavailable".into(),
            ));
        }
        let client_id = match (&self.client_id, params.remove("client_id")) {
            (Some(expected), Some(received)) if expected != &received => {
                return Err(AuthError::Invalid("callback client ID changed".into()));
            }
            (Some(expected), _) => expected.clone(),
            (None, Some(received))
                if !received.is_empty() && received != "dynamic_agent_client" =>
            {
                received
            }
            _ => {
                return Err(AuthError::Invalid(
                    "registration did not return an issued client ID".into(),
                ));
            }
        };
        let code = params
            .remove("code")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| AuthError::Invalid("callback did not contain a code".into()))?;
        Ok((code, client_id))
    }
}

fn valid_host(host: &str) -> bool {
    host.strip_prefix("urn:uuid:")
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .is_some_and(|value| value.get_version_num() == 4 && value.urn().to_string() == host)
}

fn prepare_host(directory: &Path, registered: bool) -> Result<String, AuthError> {
    let host_path = directory.join("host.json");
    let mut host: String = if host_path.exists() {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&host_path)?;
        serde_json::from_reader(file)
            .map_err(|_| AuthError::Invalid("invalid Astrid host identifier".into()))?
    } else {
        let host = uuid::Uuid::new_v4().urn().to_string();
        save(&host_path, &host)?;
        host
    };
    if !valid_host(&host) {
        if registered {
            return Err(AuthError::Invalid(
                "registered host identifier is invalid; credentials retained".into(),
            ));
        }
        // Repair the rejected pre-registration format without replacing any
        // identity that has successfully registered with OpenAI.
        host = uuid::Uuid::new_v4().urn().to_string();
        save(&host_path, &host)?;
    }
    Ok(host)
}

/// Notify the caller of the URL; the user performs sign-in in their browser.
pub async fn login(
    directory: &Path,
    notify: impl FnOnce(&str) -> io::Result<()>,
) -> Result<(), AuthError> {
    protected_directory(directory)?;
    let _lock = lock(directory).await?;
    let previous = load(&directory.join("credentials.json"))?;
    let host = prepare_host(directory, previous.is_some())?;
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let attempt = LoginAttempt {
        state: random(),
        nonce: random(),
        verifier: random(),
        redirect_uri: format!(
            "http://127.0.0.1:{}/auth/callback",
            listener.local_addr()?.port()
        ),
        client_id: previous.as_ref().map(|c| c.client_id.clone()),
    };
    notify(attempt.url(&host)?.as_str())?;
    let (code,client_id)=tokio::time::timeout(Duration::from_secs(300), async {
        loop {
            let (mut stream,_)=listener.accept().await?;
            let mut header=Vec::new();
            let read=tokio::time::timeout(Duration::from_secs(5),async {
                let mut buffer=[0u8;1024];
                while !header.windows(4).any(|v|v==b"\r\n\r\n") && header.len()<8192 {
                    let size=stream.read(&mut buffer).await?;
                    if size==0 { break; } header.extend_from_slice(&buffer[..size]);
                }
                Ok::<_,io::Error>(())
            }).await;
            if !matches!(read,Ok(Ok(()))) { continue; }
            let first=std::str::from_utf8(&header).ok().and_then(|v|v.lines().next()).unwrap_or("");
            let mut parts=first.split_whitespace();
            let result=if parts.next()==Some("GET") { attempt.callback(parts.next().unwrap_or("")) }
                else { Err(AuthError::Invalid("expected GET callback".into())) };
            let success=result.is_ok();
            let body=if success { "Authorization received. Return to Astrid to finish validation." } else { "Authorization failed. Return to Astrid and retry sign-in." };
            let status=if success { "200 OK" } else { "400 Bad Request" };
            let response=format!("HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len());
            let _=stream.write_all(response.as_bytes()).await;
            // One callback consumes this authorization attempt; never reuse a code.
            return result;
        }
    }).await.map_err(|_|AuthError::Invalid("ChatGPT sign-in timed out after 5 minutes".into()))??;
    let client = client()?;
    let tokens = token_request(
        &client,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &client_id),
            ("code", &code),
            ("code_verifier", &attempt.verifier),
            ("redirect_uri", &attempt.redirect_uri),
            ("resource", RESOURCE),
        ],
    )
    .await?;
    let id_token = tokens
        .id_token
        .ok_or_else(|| AuthError::Invalid("token response has no ID token".into()))?;
    let identity = verify_identity(
        &id_token,
        &client_id,
        Some(&attempt.nonce),
        &jwks(&client).await?,
    )?;
    if previous
        .as_ref()
        .is_some_and(|saved| saved.subject != identity.sub)
    {
        return Err(AuthError::Invalid(
            "sign-in returned a different account; existing credentials retained".into(),
        ));
    }
    let scopes: Vec<String> = tokens
        .scope
        .ok_or_else(|| AuthError::Invalid("token response has no granted scopes".into()))?
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    require_plan(&scopes)?;
    let saved = Credentials {
        issuer: ISSUER.into(),
        client_id,
        subject: identity.sub,
        email: identity.email,
        ext_agent_host_id: host,
        id_token,
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .ok_or_else(|| AuthError::Invalid("no renewable session was granted".into()))?,
        scopes,
        expires_at: now()?
            .checked_add(tokens.expires_in)
            .ok_or_else(|| AuthError::Invalid("invalid token expiry".into()))?,
    };
    save(&directory.join("credentials.json"), &saved)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_ids_use_stable_supported_uuid_uris() {
        let host = uuid::Uuid::new_v4().urn().to_string();
        assert!(valid_host(&host));
        assert!(!valid_host("astrid-random"));
        assert!(!valid_host("urn:uuid:invalid"));
        assert!(!valid_host("urn:uuid:00000000-0000-0000-0000-000000000000"));
    }
    #[test]
    fn rejected_host_format_is_repaired_only_before_registration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("host.json");
        save(&path, &"astrid-rejected").unwrap();
        assert!(prepare_host(directory.path(), true).is_err());
        assert_eq!(
            serde_json::from_reader::<_, String>(File::open(&path).unwrap()).unwrap(),
            "astrid-rejected"
        );
        let repaired = prepare_host(directory.path(), false).unwrap();
        assert!(valid_host(&repaired));
        assert_eq!(prepare_host(directory.path(), false).unwrap(), repaired);
        assert_eq!(prepare_host(directory.path(), true).unwrap(), repaired);
    }
    #[test]
    fn id_tokens_require_valid_signature_issuer_audience_expiry_and_nonce() {
        use jsonwebtoken::{EncodingKey, Header, encode};
        let jwks: JwkSet =
            serde_json::from_str(include_str!("../tests/fixtures/auth/test-only-jwks.json"))
                .unwrap();
        let key = EncodingKey::from_rsa_pem(include_bytes!(
            "../tests/fixtures/auth/test-only-private.pem"
        ))
        .unwrap();
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key".into());
        let valid = serde_json::json!({"iss":ISSUER,"aud":"issued","sub":"account","exp":now().unwrap()+3600,"iat":now().unwrap(),"nonce":"expected","email":"test@example.invalid"});
        let token = encode(&header, &valid, &key).unwrap();
        assert_eq!(
            verify_identity(&token, "issued", Some("expected"), &jwks)
                .unwrap()
                .sub,
            "account"
        );
        assert!(verify_identity(&token, "other", Some("expected"), &jwks).is_err());
        assert!(verify_identity(&token, "issued", Some("wrong"), &jwks).is_err());
        for (field, value) in [
            ("iss", serde_json::json!("https://example.invalid")),
            ("exp", serde_json::json!(1)),
            ("sub", serde_json::json!("")),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(
                verify_identity(
                    &encode(&header, &invalid, &key).unwrap(),
                    "issued",
                    Some("expected"),
                    &jwks
                )
                .is_err()
            );
        }
        let mut tampered = token.into_bytes();
        let signature = tampered.iter().rposition(|byte| *byte == b'.').unwrap() + 1;
        tampered[signature] = if tampered[signature] == b'A' {
            b'B'
        } else {
            b'A'
        };
        assert!(
            verify_identity(
                std::str::from_utf8(&tampered).unwrap(),
                "issued",
                Some("expected"),
                &jwks
            )
            .is_err()
        );
        let mut insecure = Header::new(Algorithm::HS256);
        insecure.kid = Some("test-key".into());
        assert!(
            verify_identity(
                &encode(&insecure, &valid, &EncodingKey::from_secret(b"test-only")).unwrap(),
                "issued",
                Some("expected"),
                &jwks
            )
            .is_err()
        );
    }
    fn attempt() -> LoginAttempt {
        LoginAttempt {
            state: "expected".into(),
            nonce: "nonce".into(),
            verifier: "verifier".into(),
            redirect_uri: "http://127.0.0.1:1234/auth/callback".into(),
            client_id: None,
        }
    }
    #[test]
    fn registration_validates_state_error_and_issued_client() {
        let pending = attempt();
        assert!(
            pending
                .callback("/auth/callback?state=wrong&code=x&client_id=issued")
                .is_err()
        );
        assert!(
            pending
                .callback("/auth/callback?state=expected&error=access_denied")
                .is_err()
        );
        assert!(
            pending
                .callback("/auth/callback?state=expected&code=x")
                .is_err()
        );
        assert!(
            pending
                .callback("/auth/callback?state=expected&code=x&client_id=dynamic_agent_client")
                .is_err()
        );
        assert!(
            pending
                .callback("/auth/callback?state=expected&state=expected&code=x&client_id=issued")
                .is_err()
        );
        assert_eq!(
            pending
                .callback("/auth/callback?state=expected&code=x&client_id=issued")
                .unwrap(),
            ("x".into(), "issued".into())
        );
    }
    #[test]
    fn reauthorization_cannot_replace_client_identity() {
        let mut pending = attempt();
        pending.client_id = Some("issued".into());
        assert!(
            pending
                .callback("/auth/callback?state=expected&code=x&client_id=other")
                .is_err()
        );
        assert_eq!(
            pending
                .callback("/auth/callback?state=expected&code=x")
                .unwrap()
                .1,
            "issued"
        );
    }
    #[test]
    fn registration_url_has_pkce_nonce_and_plan_scopes() {
        let pending = attempt();
        let url = pending.url("host").unwrap();
        let pairs = url.query_pairs().collect::<HashMap<_, _>>();
        assert_eq!(pairs["client_id"], "dynamic_agent_client");
        assert_eq!(pairs["agent_name_hint"], "Astrid");
        assert_eq!(pairs["resource"], RESOURCE);
        assert_eq!(pairs["nonce"], "nonce");
        assert_eq!(
            pairs["code_challenge"],
            URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier"))
        );
        assert!(pairs["scope"].contains("chatgpt.tokens.use.direct"));
    }
    #[test]
    fn identity_only_grant_cannot_authorize_inference() {
        assert!(require_plan(&["openid".into()]).is_err());
        assert!(
            require_plan(&["resource.invoke".into(), "chatgpt.tokens.use.direct".into()]).is_ok()
        );
    }
    #[test]
    fn credential_files_are_owner_only() {
        let directory = tempfile::tempdir().unwrap();
        save(&directory.path().join("example.json"), &json_value()).unwrap();
        assert_eq!(
            fs::metadata(directory.path().join("example.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    fn json_value() -> serde_json::Value {
        serde_json::json!({"test":"not a credential"})
    }
}
