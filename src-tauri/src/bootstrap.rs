use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};
use std::{
    fs,
    path::{Path, PathBuf},
};

const LEGACY_BOOTSTRAP_FILE_NAME: &str = ".bootstrap-admin.local";
const BOOTSTRAP_PASSWORD_ENV: &str = "MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD";
const BOOTSTRAP_USERNAME_ENV: &str = "MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_USERNAME";

#[derive(Debug, Clone)]
pub struct BootstrapCredential {
    pub username: String,
    pub password: String,
    pub configured_by_environment: bool,
}

/// Resolve a credencial inicial exclusivamente em memória.
///
/// A senha nunca é persistida em arquivo e nunca deve ser retornada ao frontend.
/// Para instalações headless, o operador pode definir
/// MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD antes da primeira inicialização.
pub fn resolve_internal() -> Result<BootstrapCredential, String> {
    let username = std::env::var(BOOTSTRAP_USERNAME_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_lowercase())
        .unwrap_or_else(|| "admin".to_string());

    let environment_password = std::env::var(BOOTSTRAP_PASSWORD_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let configured_by_environment = environment_password.is_some();
    let password = environment_password.unwrap_or_else(|| {
        OsRng
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect()
    });

    Ok(BootstrapCredential {
        username,
        password,
        configured_by_environment,
    })
}

pub fn environment_password_configured() -> bool {
    std::env::var(BOOTSTRAP_PASSWORD_ENV)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

/// Remove somente o arquivo legado criado por versões anteriores da template.
pub fn remove_legacy_file(db_path: &Path) -> Result<(), String> {
    for path in legacy_candidate_paths(db_path)? {
        if !path.is_file() {
            continue;
        }
        fs::remove_file(&path).map_err(|err| {
            format!(
                "A credencial legada não pôde ser removida de {}: {err}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn legacy_candidate_paths(db_path: &Path) -> Result<Vec<PathBuf>, String> {
    let data_dir = db_path
        .parent()
        .ok_or_else(|| "Diretório do banco não pôde ser determinado.".to_string())?;
    Ok(vec![data_dir.join(LEGACY_BOOTSTRAP_FILE_NAME)])
}
