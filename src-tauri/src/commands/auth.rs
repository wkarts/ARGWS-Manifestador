use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use tauri::State;
use uuid::Uuid;

use crate::{
    app_state::SharedState,
    bootstrap,
    db::{open_connection, write_app_log, AppLogInput},
    models::{AuthUser, LoginResponse, SessionIdentity},
    security::{hash_password, load_password_policy, verify_password},
};

pub(crate) const PERMISSION_CATALOG: [(&str, &str); 21] = [
    ("dashboard:view", "Visualizar dashboard"),
    ("empresas:view", "Visualizar empresas"),
    ("empresas:manage", "Gerenciar empresas"),
    ("cadastros:view", "Visualizar cadastros genéricos"),
    ("cadastros:manage", "Gerenciar cadastros genéricos"),
    ("usuarios:view", "Visualizar usuários"),
    ("usuarios:manage", "Gerenciar usuários"),
    ("perfis:view", "Visualizar perfis de acesso"),
    ("perfis:manage", "Gerenciar perfis de acesso"),
    ("config:view", "Visualizar configurações"),
    ("config:manage", "Gerenciar configurações"),
    ("licenciamento:view", "Visualizar licenciamento"),
    ("licenciamento:manage", "Gerenciar licenciamento"),
    ("logs:view", "Visualizar logs"),
    ("logs:manage", "Gerenciar logs"),
    ("sync:view", "Visualizar fila de sincronização"),
    ("relatorios:export", "Exportar relatórios"),
    ("manifestador:view", "Visualizar Manifestador de NF-e"),
    ("manifestador:manage", "Operar Manifestador de NF-e"),
    ("api:access", "Acessar a aplicação pela API"),
    ("mobile:access", "Acessar a aplicação mobile"),
];

pub(crate) fn all_permission_keys() -> Vec<String> {
    let mut keys = PERMISSION_CATALOG
        .iter()
        .map(|(key, _)| (*key).to_string())
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn permission_keys_for_user(
    conn: &Connection,
    user_id: i64,
    master: bool,
) -> Result<Vec<String>, String> {
    if master {
        return Ok(all_permission_keys());
    }
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT pp.permissao_chave
             FROM usuarios_perfis up
             INNER JOIN perfis_permissoes pp ON pp.perfil_id = up.perfil_id
             WHERE up.usuario_id = ?1
             ORDER BY pp.permissao_chave ASC",
        )
        .map_err(|err| format!("Falha ao preparar permissões do usuário: {err}"))?;
    let permissions = stmt
        .query_map([user_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("Falha ao consultar permissões do usuário: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear permissões do usuário: {err}"))?;
    Ok(permissions)
}

pub(crate) fn has_permission(
    conn: &Connection,
    identity: &SessionIdentity,
    permission: &str,
) -> Result<bool, String> {
    if identity.master_user {
        return Ok(true);
    }
    Ok(permission_keys_for_user(conn, identity.user_id, false)?
        .iter()
        .any(|key| key == permission))
}

pub(crate) fn build_auth_user_for_channel(
    conn: &Connection,
    user_id: i64,
    channel: Option<String>,
) -> Result<AuthUser, String> {
    let base = conn
        .query_row(
            "SELECT id, nome, login, email, telefone, cargo, photo_url,
                    administrador, master_user, senha_provisoria, ativo,
                    api_access_enabled, mobile_access_enabled,
                    api_session_timeout_minutes, local_guest
             FROM usuarios
             WHERE id = ?1
             LIMIT 1",
            [user_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar usuário autenticado: {err}"))?
        .ok_or_else(|| "Usuário da sessão não encontrado.".to_string())?;

    if base.10 == 0 {
        return Err("Usuário inativo.".to_string());
    }

    let mut stmt_profiles = conn
        .prepare(
            "SELECT p.nome
             FROM usuarios_perfis up
             INNER JOIN perfis_acesso p ON p.id = up.perfil_id
             WHERE up.usuario_id = ?1
             ORDER BY p.nome ASC",
        )
        .map_err(|err| format!("Falha ao preparar perfis do usuário: {err}"))?;
    let profile_names = stmt_profiles
        .query_map([user_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("Falha ao consultar perfis do usuário: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear perfis do usuário: {err}"))?;

    let mut stmt_companies = conn
        .prepare(
            "SELECT e.id, e.nome
             FROM usuarios_empresas ue
             INNER JOIN empresas e ON e.id = ue.empresa_id
             WHERE ue.usuario_id = ?1
             ORDER BY e.nome ASC",
        )
        .map_err(|err| format!("Falha ao preparar empresas do usuário: {err}"))?;
    let company_rows = stmt_companies
        .query_map([user_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| format!("Falha ao consultar empresas do usuário: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear empresas do usuário: {err}"))?;

    Ok(AuthUser {
        id: base.0,
        nome: base.1,
        login: base.2,
        email: base.3,
        telefone: base.4,
        cargo: base.5,
        photo_url: base.6,
        administrador: base.7 == 1,
        master_user: base.8 == 1,
        senha_provisoria: base.9 == 1,
        api_access_enabled: base.11 == 1,
        mobile_access_enabled: base.12 == 1,
        api_session_timeout_minutes: base.13.clamp(15, 43_200),
        local_guest: base.14 == 1,
        session_channel: channel,
        permission_keys: if base.14 == 1 {
            vec![
                "dashboard:view".to_string(),
                "empresas:view".to_string(),
                "empresas:manage".to_string(),
                "cadastros:view".to_string(),
                "cadastros:manage".to_string(),
                "sync:view".to_string(),
                "relatorios:export".to_string(),
                "mobile:access".to_string(),
            ]
        } else {
            permission_keys_for_user(conn, user_id, base.8 == 1)?
        },
        profile_names,
        company_ids: company_rows.iter().map(|item| item.0).collect(),
        company_names: company_rows.into_iter().map(|item| item.1).collect(),
    })
}

fn validate_channel_access(conn: &Connection, identity: &SessionIdentity) -> Result<(), String> {
    if identity.master_user {
        return Ok(());
    }
    match identity.channel.as_str() {
        "api" if !identity.api_access_enabled || !has_permission(conn, identity, "api:access")? => {
            return Err("Acesso pela API não está habilitado para este usuário.".to_string());
        }
        "mobile" => {
            if !identity.api_access_enabled || !has_permission(conn, identity, "api:access")? {
                return Err("Acesso pela API não está habilitado para este usuário.".to_string());
            }
            if !identity.mobile_access_enabled || !has_permission(conn, identity, "mobile:access")?
            {
                return Err("Acesso mobile não está habilitado para este usuário.".to_string());
            }
        }
        "mobile-local"
            if !identity.mobile_access_enabled
                || (!identity.local_guest && !has_permission(conn, identity, "mobile:access")?) =>
        {
            return Err("Acesso mobile não está habilitado para este usuário.".to_string());
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn load_session_identity(
    conn: &Connection,
    session_token: &str,
) -> Result<SessionIdentity, String> {
    let token = session_token.trim();
    if token.is_empty() {
        return Err("Sessão inválida ou expirada.".to_string());
    }
    let now = Utc::now().to_rfc3339();
    let session = conn
        .query_row(
            "SELECT us.usuario_id, u.master_user, u.ativo, u.senha_provisoria,
                    u.api_access_enabled, u.mobile_access_enabled, u.local_guest,
                    COALESCE(us.channel, 'desktop')
             FROM user_sessions us
             INNER JOIN usuarios u ON u.id = us.usuario_id
             WHERE us.session_token = ?1 AND us.expires_at > ?2
             LIMIT 1",
            params![token, now],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("Falha ao validar sessão: {err}"))?
        .ok_or_else(|| "Sessão inválida ou expirada.".to_string())?;

    if session.2 == 0 {
        return Err("Usuário inativo.".to_string());
    }
    conn.execute(
        "UPDATE user_sessions SET last_activity_at = ?1 WHERE session_token = ?2",
        params![now, token],
    )
    .map_err(|err| format!("Falha ao atualizar atividade da sessão: {err}"))?;

    let identity = SessionIdentity {
        user_id: session.0,
        master_user: session.1 == 1,
        senha_provisoria: session.3 == 1,
        api_access_enabled: session.4 == 1,
        mobile_access_enabled: session.5 == 1,
        local_guest: session.6 == 1,
        channel: session.7,
    };
    validate_channel_access(conn, &identity)?;
    Ok(identity)
}

pub(crate) fn require_session_by_token(
    conn: &Connection,
    session_token: &str,
) -> Result<SessionIdentity, String> {
    let identity = load_session_identity(conn, session_token)?;
    if identity.senha_provisoria {
        return Err("Troque a senha provisória antes de acessar este recurso.".to_string());
    }
    Ok(identity)
}

fn normalized_channel(channel: &str) -> &'static str {
    match channel.trim().to_ascii_lowercase().as_str() {
        "api" => "api",
        "mobile" | "remote-mobile" => "mobile",
        "mobile-local" | "local-mobile" => "mobile-local",
        _ => "desktop",
    }
}

fn login_failure(message: &str) -> LoginResponse {
    LoginResponse {
        success: false,
        message: message.to_string(),
        session_token: None,
        user: None,
    }
}

fn session_duration(conn: &Connection, channel: &str, user_timeout_minutes: i64) -> Duration {
    if matches!(channel, "api" | "mobile") {
        return Duration::minutes(user_timeout_minutes.clamp(15, 43_200));
    }
    let days = conn
        .query_row(
            "SELECT valor FROM app_settings WHERE chave = 'login_session_days' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(7)
        .clamp(1, 30);
    Duration::days(days)
}

pub(crate) fn login_with_channel(
    conn: &Connection,
    login: &str,
    password: &str,
    channel: &str,
) -> Result<LoginResponse, String> {
    let channel = normalized_channel(channel);
    let normalized_login = login.trim().to_lowercase();
    let row = conn
        .query_row(
            "SELECT id, login, senha_hash, ativo, master_user, senha_provisoria,
                    api_access_enabled, mobile_access_enabled, api_session_timeout_minutes,
                    local_guest
             FROM usuarios
             WHERE LOWER(login) = ?1 AND local_guest = 0
             LIMIT 1",
            [normalized_login],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar usuário: {err}"))?;

    let Some(row) = row else {
        return Ok(login_failure("Credenciais inválidas."));
    };
    if row.3 == 0 || !verify_password(password, &row.2)? {
        return Ok(login_failure("Credenciais inválidas."));
    }

    let provisional_identity = SessionIdentity {
        user_id: row.0,
        master_user: row.4 == 1,
        senha_provisoria: row.5 == 1,
        api_access_enabled: row.6 == 1,
        mobile_access_enabled: row.7 == 1,
        local_guest: row.9 == 1,
        channel: channel.to_string(),
    };
    if let Err(message) = validate_channel_access(conn, &provisional_identity) {
        return Ok(login_failure(&message));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let session_token = Uuid::new_v4().to_string();
    let expires_at = (now + session_duration(conn, channel, row.8)).to_rfc3339();
    conn.execute(
        "DELETE FROM user_sessions WHERE (usuario_id = ?1 AND channel = ?2) OR expires_at <= ?3",
        params![row.0, channel, now_str],
    )
    .map_err(|err| format!("Falha ao limpar sessões antigas: {err}"))?;
    conn.execute(
        "INSERT INTO user_sessions (
            usuario_id, session_token, created_at, expires_at, last_activity_at, channel
         ) VALUES (?1, ?2, ?3, ?4, ?3, ?5)",
        params![row.0, session_token, now_str, expires_at, channel],
    )
    .map_err(|err| format!("Falha ao criar sessão: {err}"))?;
    conn.execute(
        "UPDATE usuarios SET ultimo_login_em = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), row.0],
    )
    .map_err(|err| format!("Falha ao atualizar último login: {err}"))?;

    let user = build_auth_user_for_channel(conn, row.0, Some(channel.to_string()))?;
    Ok(LoginResponse {
        success: true,
        message: if user.senha_provisoria {
            "Login efetuado. Defina sua senha definitiva para continuar.".to_string()
        } else {
            "Login efetuado com sucesso.".to_string()
        },
        session_token: Some(session_token),
        user: Some(user),
    })
}

#[allow(dead_code)]
#[cfg(feature = "mobile")]
fn create_local_guest_session(conn: &Connection) -> Result<LoginResponse, String> {
    let guest_enabled = conn
        .query_row(
            "SELECT valor FROM app_settings WHERE chave = 'mobile_guest_access_enabled' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar acesso local mobile: {err}"))?
        .is_none_or(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "sim"
            )
        });
    if !guest_enabled {
        return Ok(login_failure("O acesso local sem login está desabilitado."));
    }
    let user_id = conn
        .query_row(
            "SELECT id FROM usuarios WHERE local_guest = 1 AND ativo = 1 LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao localizar usuário local mobile: {err}"))?
        .ok_or_else(|| "Usuário local mobile não foi inicializado.".to_string())?;
    let now = Utc::now();
    let token = Uuid::new_v4().to_string();
    conn.execute(
        "DELETE FROM user_sessions WHERE usuario_id = ?1 OR expires_at <= ?2",
        params![user_id, now.to_rfc3339()],
    )
    .map_err(|err| format!("Falha ao limpar sessão local anterior: {err}"))?;
    conn.execute(
        "INSERT INTO user_sessions (usuario_id, session_token, created_at, expires_at, last_activity_at, channel)
         VALUES (?1, ?2, ?3, ?4, ?3, 'mobile-local')",
        params![user_id, token, now.to_rfc3339(), (now + Duration::days(30)).to_rfc3339()],
    )
    .map_err(|err| format!("Falha ao criar sessão local mobile: {err}"))?;
    let user = build_auth_user_for_channel(conn, user_id, Some("mobile-local".to_string()))?;
    Ok(LoginResponse {
        success: true,
        message: "Modo local iniciado sem login.".to_string(),
        session_token: Some(token),
        user: Some(user),
    })
}

pub(crate) fn bootstrap_status_value(conn: &Connection) -> Result<Value, String> {
    let row = conn
        .query_row(
            "SELECT login, senha_provisoria FROM usuarios WHERE master_user = 1 ORDER BY id ASC LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar estado do usuário master: {err}"))?;
    let policy = load_password_policy(conn)?;
    Ok(json!({
        "requires_setup": row.as_ref().is_some_and(|item| item.1 == 1),
        "username": row.map(|item| item.0).unwrap_or_else(|| "admin".to_string()),
        "password_policy": policy.to_public_value(),
        "environment_password_configured": bootstrap::environment_password_configured(),
        "legacy_file_detected": false,
    }))
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_password_policy(state: State<'_, SharedState>) -> Result<Value, String> {
    let conn = open_connection(&state.db_path()?)?;
    Ok(load_password_policy(&conn)?.to_public_value())
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_bootstrap_status(state: State<'_, SharedState>) -> Result<Value, String> {
    let conn = open_connection(&state.db_path()?)?;
    bootstrap_status_value(&conn)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_initialize_master_password(
    state: State<'_, SharedState>,
    new_password: String,
) -> Result<LoginResponse, String> {
    let db_path = state.db_path()?;
    let conn = open_connection(&db_path)?;
    let policy = load_password_policy(&conn)?;
    policy.validate(new_password.trim())?;
    let row = conn
        .query_row(
            "SELECT id, login, senha_provisoria FROM usuarios WHERE master_user = 1 ORDER BY id ASC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar usuário master: {err}"))?
        .ok_or_else(|| "Usuário master não encontrado.".to_string())?;
    if row.2 == 0 {
        return Err("O primeiro acesso já foi concluído.".to_string());
    }
    conn.execute(
        "UPDATE usuarios SET senha_hash = ?1, senha_provisoria = 0, updated_at = ?2 WHERE id = ?3",
        params![
            hash_password(new_password.trim())?,
            Utc::now().to_rfc3339(),
            row.0
        ],
    )
    .map_err(|err| format!("Falha ao definir senha inicial: {err}"))?;
    bootstrap::remove_legacy_file(&db_path)?;
    let channel = if cfg!(all(feature = "mobile", not(feature = "desktop"))) {
        "mobile-local"
    } else {
        "desktop"
    };
    login_with_channel(&conn, &row.1, new_password.trim(), channel)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_login(
    state: State<'_, SharedState>,
    login: String,
    senha: String,
    channel: Option<String>,
) -> Result<LoginResponse, String> {
    let db_path = state.db_path()?;
    let data_dir = state.data_dir()?;
    let conn = open_connection(&db_path)?;
    let default_channel = if cfg!(all(feature = "mobile", not(feature = "desktop"))) {
        "mobile-local"
    } else {
        "desktop"
    };
    let channel = channel.as_deref().unwrap_or(default_channel);
    let response = login_with_channel(&conn, &login, &senha, channel)?;
    let _ = write_app_log(
        &conn,
        &data_dir,
        AppLogInput {
            level: if response.success { "info" } else { "warning" },
            category: "auth",
            message: if response.success {
                "Login efetuado com sucesso."
            } else {
                "Tentativa de login rejeitada."
            },
            source: Some("backend"),
            route: None,
            details: Some(&json!({"login": login.trim(), "channel": normalized_channel(channel)})),
        },
    );
    Ok(response)
}

#[allow(dead_code)]
#[cfg(feature = "mobile")]
#[tauri::command]
pub fn auth_local_guest(state: State<'_, SharedState>) -> Result<LoginResponse, String> {
    let conn = open_connection(&state.db_path()?)?;
    create_local_guest_session(&conn)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_restore(
    state: State<'_, SharedState>,
    session_token: String,
) -> Result<LoginResponse, String> {
    let conn = open_connection(&state.db_path()?)?;
    match load_session_identity(&conn, &session_token) {
        Ok(identity) => Ok(LoginResponse {
            success: true,
            message: "Sessão restaurada com sucesso.".to_string(),
            session_token: Some(session_token),
            user: Some(build_auth_user_for_channel(
                &conn,
                identity.user_id,
                Some(identity.channel),
            )?),
        }),
        Err(_) => Ok(login_failure("Sessão inválida ou expirada.")),
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_logout(state: State<'_, SharedState>, session_token: String) -> Result<bool, String> {
    let conn = open_connection(&state.db_path()?)?;
    conn.execute(
        "DELETE FROM user_sessions WHERE session_token = ?1",
        [session_token],
    )
    .map_err(|err| format!("Falha ao encerrar sessão: {err}"))?;
    Ok(true)
}

pub(crate) fn change_password_with_connection(
    conn: &Connection,
    db_path: &std::path::Path,
    session_token: &str,
    current_password: &str,
    new_password: &str,
) -> Result<bool, String> {
    let identity = load_session_identity(conn, session_token)?;
    load_password_policy(conn)?.validate(new_password.trim())?;
    let current_hash = conn
        .query_row(
            "SELECT senha_hash FROM usuarios WHERE id = ?1 LIMIT 1",
            [identity.user_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|err| format!("Falha ao consultar senha atual: {err}"))?;
    if !verify_password(current_password.trim(), &current_hash)? {
        return Err("A senha atual informada está incorreta.".to_string());
    }
    conn.execute(
        "UPDATE usuarios SET senha_hash = ?1, senha_provisoria = 0, updated_at = ?2 WHERE id = ?3",
        params![
            hash_password(new_password.trim())?,
            Utc::now().to_rfc3339(),
            identity.user_id
        ],
    )
    .map_err(|err| format!("Falha ao atualizar senha: {err}"))?;
    if identity.master_user {
        bootstrap::remove_legacy_file(db_path)?;
    }
    Ok(true)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[tauri::command]
pub fn auth_change_password(
    state: State<'_, SharedState>,
    session_token: String,
    current_password: String,
    new_password: String,
) -> Result<bool, String> {
    let db_path = state.db_path()?;
    let conn = open_connection(&db_path)?;
    change_password_with_connection(
        &conn,
        &db_path,
        &session_token,
        &current_password,
        &new_password,
    )
}

fn required_permission(command: &str) -> Option<&'static str> {
    match command {
        "dashboard_stats" => Some("dashboard:view"),
        "company_list" | "company_get" => Some("empresas:view"),
        "company_save" | "company_delete" | "company_lookup_cnpj" | "company_lookup_ie" => {
            Some("empresas:manage")
        }
        "entity_list"
        | "entity_get"
        | "combo_list"
        | "entity_provider_list"
        | "provider_entity_list"
        | "entity_provider_list_data"
        | "provider_entity_get" => Some("cadastros:view"),
        "entity_save"
        | "entity_delete"
        | "provider_entity_create"
        | "provider_entity_update"
        | "provider_entity_delete" => Some("cadastros:manage"),
        "user_list" | "user_get" | "user_policy_get" => Some("usuarios:view"),
        "user_save" | "user_delete" | "user_policy_save" => Some("usuarios:manage"),
        "profile_list" | "profile_get" | "permission_catalog" => Some("perfis:view"),
        "profile_save" | "profile_delete" => Some("perfis:manage"),
        "app_log_list" => Some("logs:view"),
        "app_log_clear" => Some("logs:manage"),
        "sync_queue_list" | "sync_queue_mark_synced" => Some("sync:view"),
        "licensing_load_settings"
        | "licensing_status"
        | "licensing_device_info"
        | "licensing_check_runtime"
        | "support_guard_status" => Some("licenciamento:view"),
        "licensing_save_settings"
        | "licensing_start_trial"
        | "support_guard_provision"
        | "support_guard_enable_totp"
        | "support_guard_unlock" => Some("licenciamento:manage"),
        "integration_list" | "integration_test" | "integration_logs" => Some("config:view"),
        "integration_save" | "integration_delete" => Some("config:manage"),
        "runtime_settings_load"
        | "runtime_env_example"
        | "internal_api_status"
        | "internal_api_test"
        | "internal_api_test_port"
        | "web_proxy_status"
        | "webhook_status"
        | "webhook_list_events"
        | "websocket_status"
        | "websocket_list_clients"
        | "app_service_status"
        | "tray_status"
        | "provider_list_with_database" => Some("config:view"),
        "runtime_settings_save"
        | "internal_api_start"
        | "internal_api_restart"
        | "internal_api_stop"
        | "web_proxy_start"
        | "web_proxy_restart"
        | "web_proxy_stop"
        | "webhook_start"
        | "webhook_restart"
        | "webhook_stop"
        | "webhook_clear_events"
        | "webhook_test_receive"
        | "websocket_start"
        | "websocket_restart"
        | "websocket_stop"
        | "websocket_broadcast_test"
        | "startup_with_windows_set"
        | "app_service_install"
        | "app_service_uninstall"
        | "app_service_start"
        | "app_service_stop"
        | "app_service_restart"
        | "tray_restore_window"
        | "tray_exit_app"
        | "system_set_data_dir" => Some("config:manage"),
        "open_print_preview" => Some("relatorios:export"),
        "application_smtp_get" => Some("config:view"),
        "application_smtp_save" => Some("config:manage"),
        "manifestador_fiscal_status" | "manifestador_fiscal_process" => Some("manifestador:view"),
        "manifestador_summary"
        | "manifestador_config_get"
        | "manifestador_company_config_get"
        | "manifestador_notes_list"
        | "manifestador_note_get"
        | "manifestador_note_logs"
        | "manifestador_provider_test" => Some("manifestador:view"),
        "manifestador_certificate_import"
        | "manifestador_config_save"
        | "manifestador_company_config_save"
        | "manifestador_import_xml"
        | "manifestador_obter_notas"
        | "manifestador_download"
        | "manifestador_manifestar"
        | "manifestador_export_zip"
        | "manifestador_send_accounting" => Some("manifestador:manage"),
        // O frontend pode registrar erros da própria sessão sem receber permissão para ler logs.
        "app_log_write" => Some("__authenticated__"),
        _ => None,
    }
}

pub(crate) fn authorize_api_command(
    conn: &Connection,
    session_token: &str,
    command: &str,
) -> Result<SessionIdentity, String> {
    let identity = require_session_by_token(conn, session_token)?;
    if !matches!(identity.channel.as_str(), "api" | "mobile") {
        return Err("A sessão informada não pertence ao canal da API.".to_string());
    }
    let permission = required_permission(command).ok_or_else(|| {
        format!("O comando {command} não está autorizado para execução pela API remota.")
    })?;
    if permission != "__authenticated__" && !has_permission(conn, &identity, permission)? {
        return Err(format!("Permissão necessária: {permission}."));
    }
    if matches!(
        command,
        "user_save" | "user_delete" | "user_policy_save" | "profile_save" | "profile_delete"
    ) && !identity.master_user
    {
        return Err(
            "Apenas o usuário master pode alterar usuários, perfis e políticas pela API."
                .to_string(),
        );
    }
    Ok(identity)
}

pub(crate) fn company_allowed(
    conn: &Connection,
    identity: &SessionIdentity,
    company_id: i64,
) -> Result<bool, String> {
    if identity.master_user {
        return Ok(true);
    }
    let exists = conn
        .query_row(
            "SELECT 1 FROM usuarios_empresas WHERE usuario_id = ?1 AND empresa_id = ?2 LIMIT 1",
            params![identity.user_id, company_id],
            |_| Ok(1_i64),
        )
        .optional()
        .map_err(|err| format!("Falha ao validar empresa do usuário: {err}"))?;
    Ok(exists.is_some())
}
