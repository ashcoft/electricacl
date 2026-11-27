//! Session authentication and Cap CAPTCHA verification.
//!
//! Credentials are stored as an Argon2 hash in `DATA_DIR/credentials.json`
//! together with the random key used to sign session cookies. The password is
//! never written to disk: it comes from `AUTH_PASSWORD`, or is generated once on
//! first boot and printed to the log.

use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const SESSION_COOKIE: &str = "eacl_session";
const CREDENTIALS_FILE: &str = "credentials.json";
const SESSION_TTL_SECONDS: i64 = 12 * 60 * 60;
const MAX_FAILURES: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(300);
const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Serialize, Deserialize)]
struct StoredCredentials {
    username: String,
    password_hash: String,
    session_secret: String,
}

#[derive(Debug, Clone)]
pub struct CapConfig {
    /// Endpoint the browser widget talks to. Absolute
    /// (`https://cap.example.com/<site>/`) or same-origin (`/cap/<site>/`).
    pub endpoint: String,
    /// Absolute URL used for the server-side verification call.
    pub verify_url: String,
    pub secret: String,
}

/// Public view of the authentication state, safe to hand to the browser.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user: Option<String>,
    pub cap_endpoint: Option<String>,
    pub cap_required: bool,
}

#[derive(Debug)]
pub enum LoginError {
    RateLimited,
    BadCredentials,
    Captcha(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for LoginError {
    fn from(error: anyhow::Error) -> Self {
        LoginError::Internal(error)
    }
}

#[derive(Default)]
struct Attempts {
    failures: u32,
    blocked_until: Option<Instant>,
}

pub struct Auth {
    username: String,
    password_hash: String,
    session_secret: Vec<u8>,
    cap: Option<CapConfig>,
    cookie_secure: bool,
    path: PathBuf,
    limiter: Mutex<HashMap<IpAddr, Attempts>>,
    /// Session ids revoked by an explicit sign-out, with their expiry so the
    /// list can be pruned.
    revoked: Mutex<HashMap<String, i64>>,
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    OsRng.fill_bytes(&mut buffer);
    buffer.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow!("hashing password: {error}"))
}

fn verify_password(hash: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

impl Auth {
    /// Load or create the credentials file.
    ///
    /// Returns the auth state plus a generated password when one was created on
    /// this boot, so the caller can log it once.
    pub fn load(
        data_dir: &Path,
        username: &str,
        password: Option<&str>,
        cap: Option<CapConfig>,
        cookie_secure: bool,
    ) -> Result<(Self, Option<String>)> {
        fs::create_dir_all(data_dir).with_context(|| format!("creating {}", data_dir.display()))?;
        let path = data_dir.join(CREDENTIALS_FILE);

        let mut generated = None;
        let stored: Option<StoredCredentials> = if path.exists() {
            let raw =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            match serde_json::from_str(&raw) {
                Ok(credentials) => Some(credentials),
                Err(error) => {
                    tracing::warn!(%error, "credentials file is corrupt, regenerating");
                    None
                }
            }
        } else {
            None
        };

        let mut credentials = match stored {
            Some(mut credentials) if password.is_none() => {
                credentials.username = username.to_string();
                credentials
            }
            Some(credentials) if password.is_some() => StoredCredentials {
                username: username.to_string(),
                password_hash: hash_password(password.expect("checked"))?,
                session_secret: credentials.session_secret,
            },
            _ => {
                let chosen = match password {
                    Some(value) if value.len() >= MIN_PASSWORD_LEN => value.to_string(),
                    Some(_) => {
                        return Err(anyhow!(
                            "AUTH_PASSWORD must be at least {MIN_PASSWORD_LEN} characters"
                        ))
                    }
                    None => {
                        let value = random_hex(12);
                        generated = Some(value.clone());
                        value
                    }
                };
                StoredCredentials {
                    username: username.to_string(),
                    password_hash: hash_password(&chosen)?,
                    session_secret: URL_SAFE_NO_PAD.encode({
                        let mut secret = [0u8; 32];
                        OsRng.fill_bytes(&mut secret);
                        secret
                    }),
                }
            }
        };
        if credentials.session_secret.is_empty() {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            credentials.session_secret = URL_SAFE_NO_PAD.encode(secret);
        }

        fs::write(&path, serde_json::to_vec_pretty(&credentials)?)
            .with_context(|| format!("writing {}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }

        let session_secret = URL_SAFE_NO_PAD
            .decode(&credentials.session_secret)
            .context("session secret is not valid base64")?;

        Ok((
            Self {
                username: credentials.username,
                password_hash: credentials.password_hash,
                session_secret,
                cap,
                cookie_secure,
                path,
                limiter: Mutex::new(HashMap::new()),
                revoked: Mutex::new(HashMap::new()),
            },
            generated,
        ))
    }

    /// Regenerate the password, print it and exit. Used for operator recovery.
    pub fn reset_password(data_dir: &Path, username: &str) -> Result<String> {
        let path = data_dir.join(CREDENTIALS_FILE);
        let session_secret = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str::<StoredCredentials>(&raw).ok())
                .map(|credentials| credentials.session_secret)
                .unwrap_or_else(|| {
                    let mut secret = [0u8; 32];
                    OsRng.fill_bytes(&mut secret);
                    URL_SAFE_NO_PAD.encode(secret)
                })
        } else {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            URL_SAFE_NO_PAD.encode(secret)
        };
        let password = random_hex(12);
        let credentials = StoredCredentials {
            username: username.to_string(),
            password_hash: hash_password(&password)?,
            session_secret,
        };
        fs::create_dir_all(data_dir)?;
        fs::write(&path, serde_json::to_vec_pretty(&credentials)?)?;
        Ok(password)
    }

    /// Replace the CAPTCHA configuration (used by the HTTP tests).
    pub fn set_cap_for_test(&mut self, cap: CapConfig) {
        self.cap = Some(cap);
    }

    pub fn cap_endpoint(&self) -> Option<String> {
        self.cap.as_ref().map(|cap| cap.endpoint.clone())
    }

    pub fn cap_required(&self) -> bool {
        self.cap.is_some()
    }

    pub fn status(&self, user: Option<&str>) -> AuthStatus {
        AuthStatus {
            authenticated: user.is_some(),
            user: user.map(str::to_string),
            cap_endpoint: self.cap_endpoint(),
            cap_required: self.cap_required(),
        }
    }

    pub fn credentials_path(&self) -> &Path {
        &self.path
    }

    fn is_blocked(&self, peer: IpAddr) -> bool {
        let Ok(mut limiter) = self.limiter.lock() else {
            return false;
        };
        let entry = limiter.entry(peer).or_default();
        if let Some(until) = entry.blocked_until {
            if Instant::now() >= until {
                *entry = Attempts::default();
                return false;
            }
            return true;
        }
        false
    }

    fn record_failure(&self, peer: IpAddr) {
        let Ok(mut limiter) = self.limiter.lock() else {
            return;
        };
        let entry = limiter.entry(peer).or_default();
        entry.failures += 1;
        if entry.failures >= MAX_FAILURES {
            entry.blocked_until = Some(Instant::now() + LOCKOUT);
            entry.failures = 0;
        }
    }

    fn record_success(&self, peer: IpAddr) {
        if let Ok(mut limiter) = self.limiter.lock() {
            limiter.remove(&peer);
        }
    }

    /// Check the username, password and (when configured) the Cap token.
    pub async fn login(
        &self,
        peer: IpAddr,
        username: &str,
        password: &str,
        cap_token: Option<&str>,
    ) -> Result<String, LoginError> {
        if self.is_blocked(peer) {
            return Err(LoginError::RateLimited);
        }

        // Always run the hash verification so a wrong username costs the same as
        // a wrong password.
        let password_ok = verify_password(&self.password_hash, password);
        let username_ok = username.trim() == self.username;
        if !password_ok || !username_ok {
            self.record_failure(peer);
            return Err(LoginError::BadCredentials);
        }

        if let Some(cap) = &self.cap {
            let token = cap_token.unwrap_or_default().trim();
            if token.is_empty() {
                self.record_failure(peer);
                return Err(LoginError::Captcha(
                    "Complete the CAPTCHA challenge before signing in".into(),
                ));
            }
            let endpoint = cap.verify_url.clone();
            let secret = cap.secret.clone();
            let token = token.to_string();
            let verified =
                tokio::task::spawn_blocking(move || verify_cap(&endpoint, &secret, &token))
                    .await
                    .map_err(|error| {
                        LoginError::Internal(anyhow!("captcha task failed: {error}"))
                    })?;
            match verified {
                Ok(true) => {}
                Ok(false) => {
                    self.record_failure(peer);
                    return Err(LoginError::Captcha(
                        "CAPTCHA verification failed, please try again".into(),
                    ));
                }
                Err(error) => return Err(LoginError::Captcha(error.to_string())),
            }
        }

        self.record_success(peer);
        Ok(self.issue_token())
    }

    fn issue_token(&self) -> String {
        let payload = serde_json::json!({
            "sub": self.username,
            "exp": now_seconds() + SESSION_TTL_SECONDS,
            "jti": random_hex(8),
        });
        let payload = URL_SAFE_NO_PAD.encode(payload.to_string());
        let signature = self.sign(payload.as_bytes());
        format!("{payload}.{signature}")
    }

    fn verify_signature(&self, payload: &[u8], signature: &str) -> Result<(), ()> {
        let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| ())?;
        let mut mac = HmacSha256::new_from_slice(&self.session_secret).map_err(|_| ())?;
        mac.update(payload);
        mac.verify_slice(&signature).map_err(|_| ())
    }

    fn sign(&self, payload: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.session_secret)
            .expect("HMAC accepts keys of any length");
        mac.update(payload);
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }

    /// Validate a session cookie and return the user name it carries.
    pub fn verify_token(&self, token: &str) -> Option<String> {
        let (payload, signature) = token.rsplit_once('.')?;
        self.verify_signature(payload.as_bytes(), signature).ok()?;

        let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        let exp = claims.get("exp")?.as_i64()?;
        if exp < now_seconds() {
            return None;
        }
        let sub = claims.get("sub")?.as_str()?;
        if sub != self.username {
            return None;
        }
        if let Some(jti) = claims.get("jti").and_then(serde_json::Value::as_str) {
            let revoked = self.revoked.lock().ok()?;
            if revoked.contains_key(jti) {
                return None;
            }
        }
        Some(sub.to_string())
    }

    /// Invalidate a token immediately, so a copied cookie stops working as soon
    /// as the user signs out. The entry is dropped once the token would have
    /// expired anyway.
    pub fn revoke(&self, token: &str) {
        let Some((payload, signature)) = token.rsplit_once('.') else {
            return;
        };
        // Only revoke a token this server actually signed.
        if self
            .verify_signature(payload.as_bytes(), signature)
            .is_err()
        {
            return;
        }
        let Ok(decoded) = URL_SAFE_NO_PAD.decode(payload) else {
            return;
        };
        let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&decoded) else {
            return;
        };
        let Some(jti) = claims.get("jti").and_then(|value| value.as_str()) else {
            return;
        };
        let exp = claims
            .get("exp")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        if exp < now_seconds() {
            return;
        }
        if let Ok(mut revoked) = self.revoked.lock() {
            let now = now_seconds();
            revoked.retain(|_, expiry| *expiry > now);
            revoked.insert(jti.to_string(), exp);
        }
    }

    pub fn session_cookie(&self, token: &str) -> String {
        let secure = if self.cookie_secure { "; Secure" } else { "" };
        format!(
            "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_SECONDS}{secure}"
        )
    }

    pub fn expired_cookie(&self) -> String {
        let secure = if self.cookie_secure { "; Secure" } else { "" };
        format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}")
    }
}

/// Read the session cookie from a request's headers.
pub fn session_from_cookie_header(header: Option<&str>) -> Option<String> {
    let header = header?;
    header.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SESSION_COOKIE).then(|| value.trim().to_string())
    })
}

fn verify_cap(endpoint: &str, secret: &str, token: &str) -> Result<bool> {
    let response = ureq::post(endpoint)
        .timeout(Duration::from_secs(8))
        .send_json(serde_json::json!({ "secret": secret, "response": token }))
        .map_err(|error| anyhow!("CAPTCHA service unreachable: {error}"))?;
    let body: serde_json::Value = response
        .into_json()
        .map_err(|error| anyhow!("CAPTCHA service returned an invalid response: {error}"))?;
    Ok(body
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_in(dir: &Path) -> Auth {
        let (auth, _) = Auth::load(dir, "admin", Some("correct-horse"), None, false).unwrap();
        auth
    }

    #[test]
    fn password_hash_roundtrip() {
        let hash = hash_password("s3cret-pass").unwrap();
        assert!(verify_password(&hash, "s3cret-pass"));
        assert!(!verify_password(&hash, "s3cret-pas"));
    }

    #[test]
    fn session_token_roundtrip_and_tamper_detection() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let auth = auth_in(&dir);
        let token = auth.issue_token();
        assert_eq!(auth.verify_token(&token).as_deref(), Some("admin"));

        let mut parts = token.split('.');
        let payload = parts.next().unwrap();
        let signature = parts.next().unwrap();
        let tampered = format!("{payload}x.{signature}");
        assert!(auth.verify_token(&tampered).is_none());
        assert!(auth.verify_token("not-a-token").is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn revoked_token_is_rejected() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let auth = auth_in(&dir);
        let token = auth.issue_token();
        assert!(auth.verify_token(&token).is_some());

        auth.revoke(&token);
        assert!(
            auth.verify_token(&token).is_none(),
            "signed-out token still valid"
        );

        // A token that was never issued by this server cannot be revoked in a
        // way that affects anything else.
        let other = auth.issue_token();
        auth.revoke("garbage.signature");
        assert!(auth.verify_token(&other).is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn generated_password_is_returned_once() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let (_, generated) = Auth::load(&dir, "admin", None, None, false).unwrap();
        let generated = generated.expect("first boot generates a password");
        assert_eq!(generated.len(), 24);

        let (_, again) = Auth::load(&dir, "admin", None, None, false).unwrap();
        assert!(again.is_none(), "password is only generated once");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cookie_parser_finds_the_session() {
        assert_eq!(
            session_from_cookie_header(Some("a=1; eacl_session=abc.def; b=2")).as_deref(),
            Some("abc.def")
        );
        assert!(session_from_cookie_header(Some("a=1")).is_none());
        assert!(session_from_cookie_header(None).is_none());
    }

    #[tokio::test]
    async fn rejects_wrong_password_and_locks_out_after_repeated_failures() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let auth = auth_in(&dir);
        let peer: IpAddr = "10.0.0.9".parse().unwrap();

        for _ in 0..MAX_FAILURES {
            assert!(matches!(
                auth.login(peer, "admin", "wrong", None).await,
                Err(LoginError::BadCredentials)
            ));
        }
        assert!(matches!(
            auth.login(peer, "admin", "correct-horse", None).await,
            Err(LoginError::RateLimited)
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn accepts_the_configured_password() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let auth = auth_in(&dir);
        let peer: IpAddr = "10.0.0.10".parse().unwrap();
        let token = auth
            .login(peer, "admin", "correct-horse", None)
            .await
            .expect("login succeeds");
        assert_eq!(auth.verify_token(&token).as_deref(), Some("admin"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn requires_a_captcha_token_when_configured() {
        let dir = std::env::temp_dir().join(format!("eacl-auth-{}", random_hex(6)));
        let (mut auth, _) = Auth::load(&dir, "admin", Some("correct-horse"), None, false).unwrap();
        auth.cap = Some(CapConfig {
            endpoint: "/cap/site/".into(),
            verify_url: "http://127.0.0.1:9/site/siteverify".into(),
            secret: "secret".into(),
        });
        let peer: IpAddr = "10.0.0.11".parse().unwrap();
        assert!(matches!(
            auth.login(peer, "admin", "correct-horse", None).await,
            Err(LoginError::Captcha(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }
}
