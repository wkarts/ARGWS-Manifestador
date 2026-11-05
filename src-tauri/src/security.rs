use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::Utc;
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| format!("Falha ao gerar hash da senha: {err}"))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|err| format!("Falha ao interpretar hash armazenado: {err}"))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn machine_key() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string());
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".to_string());
    format!("{host}:{user}")
}

fn derive_secret(seed: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"MANIFESTADOR-NFE-LIC");
    hasher.update(seed.as_bytes());
    hasher.finalize().to_vec()
}

pub fn encrypt_text(seed: &str, plain: &str) -> String {
    let secret = derive_secret(seed);
    let bytes = plain
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(idx, byte)| byte ^ secret[idx % secret.len()])
        .collect::<Vec<_>>();
    BASE64_STANDARD.encode(bytes)
}

pub fn decrypt_text(seed: &str, cipher: &str) -> Result<String, String> {
    let secret = derive_secret(seed);
    let bytes = BASE64_STANDARD
        .decode(cipher)
        .map_err(|err| format!("Falha ao decodificar conteúdo criptografado: {err}"))?;
    let plain = bytes
        .iter()
        .enumerate()
        .map(|(idx, byte)| byte ^ secret[idx % secret.len()])
        .collect::<Vec<_>>();
    String::from_utf8(plain)
        .map_err(|err| format!("Falha ao reconstruir texto criptografado: {err}"))
}

pub fn integrity_hash(seed: &str, payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.update(payload.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_support_secret() -> String {
    OsRng
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect::<String>()
}

pub fn generate_recovery_codes() -> Vec<String> {
    (0..8)
        .map(|_| {
            OsRng
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect::<String>()
        })
        .collect()
}

pub fn generate_totp_secret() -> String {
    let mut bytes = [0u8; 20];
    OsRng.fill_bytes(&mut bytes);
    BASE32_NOPAD.encode(&bytes)
}

pub fn build_otpauth_url(secret: &str, account_label: &str, issuer: &str) -> String {
    let label = account_label.replace(' ', "%20").replace('@', "%40");
    let issuer_enc = issuer.replace(' ', "%20");
    format!(
        "otpauth://totp/{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        label, secret, issuer_enc
    )
}

fn hotp(secret: &[u8], counter: u64) -> Result<u32, String> {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret)
        .map_err(|err| format!("Falha ao inicializar HMAC do TOTP: {err}"))?;
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let offset = (result[19] & 0x0f) as usize;
    let binary = ((u32::from(result[offset]) & 0x7f) << 24)
        | (u32::from(result[offset + 1]) << 16)
        | (u32::from(result[offset + 2]) << 8)
        | u32::from(result[offset + 3]);
    Ok(binary % 1_000_000)
}

pub fn verify_totp_code(secret_base32: &str, code: &str) -> Result<bool, String> {
    let normalized = code.trim().replace(' ', "");
    if normalized.len() != 6 || !normalized.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }
    let expected = normalized
        .parse::<u32>()
        .map_err(|err| format!("Falha ao interpretar código TOTP: {err}"))?;
    let secret = BASE32_NOPAD
        .decode(secret_base32.trim().as_bytes())
        .map_err(|err| format!("Falha ao decodificar secret TOTP: {err}"))?;
    let unix = chrono::Utc::now().timestamp().div_euclid(30);
    for delta in -1..=1 {
        let counter = (unix + delta) as u64;
        if hotp(&secret, counter)? == expected {
            return Ok(true);
        }
    }
    Ok(false)
}

pub const PASSWORD_MIN_ALLOWED: usize = 6;
pub const PASSWORD_MAX_ALLOWED: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_letter: bool,
    pub require_number: bool,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_symbol: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: PASSWORD_MAX_ALLOWED,
            require_letter: true,
            require_number: true,
            require_uppercase: false,
            require_lowercase: false,
            require_symbol: false,
        }
    }
}

impl PasswordPolicy {
    pub fn normalized(mut self) -> Result<Self, String> {
        if !(PASSWORD_MIN_ALLOWED..=PASSWORD_MAX_ALLOWED).contains(&self.min_length) {
            return Err(format!(
                "O tamanho mínimo da senha deve ficar entre {} e {} caracteres.",
                PASSWORD_MIN_ALLOWED, PASSWORD_MAX_ALLOWED
            ));
        }
        if self.max_length < self.min_length || self.max_length > PASSWORD_MAX_ALLOWED {
            return Err(format!(
                "O tamanho máximo da senha deve ficar entre {} e {} caracteres.",
                self.min_length, PASSWORD_MAX_ALLOWED
            ));
        }
        if self.require_uppercase || self.require_lowercase {
            self.require_letter = true;
        }
        Ok(self)
    }

    pub fn validate(&self, password: &str) -> Result<(), String> {
        let chars = password.chars().count();
        if chars < self.min_length {
            return Err(format!(
                "A senha deve conter ao menos {} caracteres.",
                self.min_length
            ));
        }
        if chars > self.max_length {
            return Err(format!(
                "A senha deve conter no máximo {} caracteres.",
                self.max_length
            ));
        }
        if self.require_letter && !password.chars().any(|value| value.is_alphabetic()) {
            return Err("A senha deve conter ao menos uma letra.".to_string());
        }
        if self.require_number && !password.chars().any(|value| value.is_ascii_digit()) {
            return Err("A senha deve conter ao menos um número.".to_string());
        }
        if self.require_uppercase && !password.chars().any(|value| value.is_uppercase()) {
            return Err("A senha deve conter ao menos uma letra maiúscula.".to_string());
        }
        if self.require_lowercase && !password.chars().any(|value| value.is_lowercase()) {
            return Err("A senha deve conter ao menos uma letra minúscula.".to_string());
        }
        if self.require_symbol && !password.chars().any(|value| !value.is_alphanumeric()) {
            return Err("A senha deve conter ao menos um símbolo.".to_string());
        }
        Ok(())
    }

    pub fn to_public_value(&self) -> Value {
        json!({
            "min_length": self.min_length,
            "max_length": self.max_length,
            "min_allowed": PASSWORD_MIN_ALLOWED,
            "max_allowed": PASSWORD_MAX_ALLOWED,
            "require_letter": self.require_letter,
            "require_number": self.require_number,
            "require_uppercase": self.require_uppercase,
            "require_lowercase": self.require_lowercase,
            "require_symbol": self.require_symbol,
        })
    }
}

const PASSWORD_POLICY_KEY: &str = "security.password_policy";

pub fn load_password_policy(conn: &Connection) -> Result<PasswordPolicy, String> {
    let raw = conn
        .query_row(
            "SELECT valor FROM app_settings WHERE chave = ?1 LIMIT 1",
            [PASSWORD_POLICY_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar política de senha: {err}"))?;

    match raw {
        Some(value) => serde_json::from_str::<PasswordPolicy>(&value)
            .map_err(|err| format!("Política de senha armazenada é inválida: {err}"))?
            .normalized(),
        None => Ok(PasswordPolicy::default()),
    }
}

pub fn save_password_policy(
    conn: &Connection,
    policy: PasswordPolicy,
) -> Result<PasswordPolicy, String> {
    let normalized = policy.normalized()?;
    let encoded = serde_json::to_string(&normalized)
        .map_err(|err| format!("Falha ao serializar política de senha: {err}"))?;
    let timestamp = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO app_settings (chave, valor, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor, updated_at = excluded.updated_at",
        params![PASSWORD_POLICY_KEY, encoded, timestamp],
    )
    .map_err(|err| format!("Falha ao salvar política de senha: {err}"))?;
    Ok(normalized)
}
