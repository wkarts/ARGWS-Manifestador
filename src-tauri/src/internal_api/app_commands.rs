use std::collections::HashSet;

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::{
    commands::{
        auth::{
            all_permission_keys, authorize_api_command, bootstrap_status_value,
            build_auth_user_for_channel, change_password_with_connection, company_allowed,
            has_permission, login_with_channel,
        },
        entities,
    },
    core::database::{config::DatabaseConfig, provider::ActiveDatabaseProvider},
    db::{open_connection, row_to_json_map, write_app_log, AppLogInput},
    internal_api::state::InternalApiState,
    security::{hash_password, load_password_policy, save_password_policy, PasswordPolicy},
};

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub command: String,
    #[serde(default)]
    pub args: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn command(
    State(state): State<InternalApiState>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, (StatusCode, Json<CommandResponse>)> {
    match execute_command(&state, &request.command, request.args) {
        Ok(result) => Ok(Json(CommandResponse {
            ok: true,
            result: Some(result),
            error: None,
        })),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(CommandResponse {
                ok: false,
                result: None,
                error: Some(error),
            }),
        )),
    }
}

fn runtime_app_name() -> String {
    std::env::var("MANIFESTADOR_NFE_NAME")
        .or_else(|_| std::env::var("APP_NAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Manifestador NF-e".to_string())
}

fn runtime_app_short_name() -> String {
    std::env::var("MANIFESTADOR_NFE_SHORT_NAME")
        .or_else(|_| std::env::var("APP_SHORT_NAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Manifestador".to_string())
}

fn runtime_app_identifier() -> String {
    std::env::var("MANIFESTADOR_NFE_IDENTIFIER")
        .or_else(|_| std::env::var("APP_IDENTIFIER"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "br.com.wwsoftwares.manifestador-nfe".to_string())
}

fn provider_for_state(state: &InternalApiState) -> Result<ActiveDatabaseProvider, String> {
    let driver = std::env::var("MANIFESTADOR_NFE_DATABASE_DRIVER")
        .or_else(|_| std::env::var("MANIFESTADOR_NFE_DB_DRIVER"))
        .unwrap_or_else(|_| "sqlite".to_string())
        .trim()
        .to_ascii_lowercase();
    let config = DatabaseConfig::from_env_with_driver(&driver);
    ActiveDatabaseProvider::from_config(config, &state.db_path)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

const USER_LOGIN_MIN_LENGTH_KEY: &str = "user_login_min_length";
const USER_LOGIN_MIN_LENGTH_DEFAULT: i64 = 2;
const USER_LOGIN_MIN_LENGTH_MIN: i64 = 1;
const USER_LOGIN_MIN_LENGTH_MAX: i64 = 64;

fn load_login_min_length(conn: &Connection) -> Result<i64, String> {
    let raw = conn
        .query_row(
            "SELECT valor FROM app_settings WHERE chave = ?1 LIMIT 1",
            [USER_LOGIN_MIN_LENGTH_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar política de login: {err}"))?;
    Ok(raw
        .as_deref()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(USER_LOGIN_MIN_LENGTH_DEFAULT)
        .clamp(USER_LOGIN_MIN_LENGTH_MIN, USER_LOGIN_MIN_LENGTH_MAX))
}

fn save_login_min_length(conn: &Connection, value: i64) -> Result<i64, String> {
    if !(USER_LOGIN_MIN_LENGTH_MIN..=USER_LOGIN_MIN_LENGTH_MAX).contains(&value) {
        return Err(format!(
            "A política de login deve ficar entre {USER_LOGIN_MIN_LENGTH_MIN} e {USER_LOGIN_MIN_LENGTH_MAX} caracteres."
        ));
    }
    let timestamp = now();
    conn.execute(
        "INSERT INTO app_settings (chave, valor, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor, updated_at = excluded.updated_at",
        params![USER_LOGIN_MIN_LENGTH_KEY, value.to_string(), timestamp],
    )
    .map_err(|err| format!("Falha ao salvar política de login: {err}"))?;
    Ok(value)
}

fn get_string(args: &Map<String, Value>, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => if *flag { "1" } else { "0" }.to_string(),
        _ => String::new(),
    }
}

fn get_i64(args: &Map<String, Value>, key: &str) -> i64 {
    args.get(key)
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.parse::<i64>().ok())
        })
        .unwrap_or_default()
}

fn validate_optional_email(value: Option<&Value>) -> Result<(), String> {
    let email = value.and_then(Value::as_str).unwrap_or_default().trim();
    if email.is_empty() || (email.contains('@') && email.contains('.')) {
        Ok(())
    } else {
        Err("O e-mail do usuário é inválido.".to_string())
    }
}

fn get_bool_value(args: &Map<String, Value>, key: &str, default: bool) -> bool {
    args.get(key)
        .and_then(|value| {
            value
                .as_bool()
                .or_else(|| value.as_i64().map(|number| number != 0))
                .or_else(|| {
                    value.as_str().map(|text| {
                        matches!(
                            text.trim().to_ascii_lowercase().as_str(),
                            "1" | "true" | "sim" | "yes"
                        )
                    })
                })
        })
        .unwrap_or(default)
}

fn get_payload(args: &Map<String, Value>) -> Map<String, Value> {
    args.get("payload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn get_filters(args: &Map<String, Value>) -> Map<String, Value> {
    args.get("filters")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn resolve_manifestador_company(
    conn: &Connection,
    identity: &crate::models::SessionIdentity,
    requested: Option<i64>,
) -> Result<Option<i64>, String> {
    if let Some(company_id) = requested.filter(|value| *value > 0) {
        if !company_allowed(conn, identity, company_id)? {
            return Err("Usuário sem vínculo com a empresa selecionada.".to_string());
        }
        let active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM empresas WHERE id = ?1 AND ativo = 1",
                [company_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("Falha ao verificar empresa ativa: {err}"))?;
        if active == 0 {
            return Err("Empresa ativa não encontrada ou inativa.".to_string());
        }
        return Ok(Some(company_id));
    }

    if identity.master_user {
        return Ok(None);
    }

    conn.query_row(
        "SELECT empresa_id
         FROM usuarios_empresas ue
         INNER JOIN empresas e ON e.id = ue.empresa_id
         WHERE ue.usuario_id = ?1 AND e.ativo = 1
         ORDER BY e.id LIMIT 1",
        [identity.user_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("Falha ao resolver empresa ativa do usuário: {err}"))?
    .map(Some)
    .ok_or_else(|| "Usuário não possui empresa ativa vinculada.".to_string())
}

fn required_manifestador_company(
    conn: &Connection,
    identity: &crate::models::SessionIdentity,
    requested: Option<i64>,
) -> Result<i64, String> {
    resolve_manifestador_company(conn, identity, requested)?
        .ok_or_else(|| "Selecione uma empresa para executar esta operação.".to_string())
}

fn search_from_args(args: &Map<String, Value>) -> String {
    let direct = get_string(args, "search");
    if !direct.is_empty() {
        return direct;
    }
    get_filters(args)
        .get("search")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn json_to_sql_value(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(v) => rusqlite::types::Value::Integer(if *v { 1 } else { 0 }),
        Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = v.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        Value::String(v) if v.trim().is_empty() => rusqlite::types::Value::Null,
        Value::String(v) => rusqlite::types::Value::Text(v.to_string()),
        _ => rusqlite::types::Value::Text(value.to_string()),
    }
}

fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn table_columns(conn: &Connection, table: &str) -> Result<HashSet<String>, String> {
    let sql = format!("PRAGMA table_info({})", quote_ident(table));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Falha ao inspecionar tabela {table}: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("Falha ao consultar colunas de {table}: {err}"))?;
    let mut columns = HashSet::new();
    for row in rows {
        columns.insert(row.map_err(|err| format!("Falha ao mapear coluna de {table}: {err}"))?);
    }
    if columns.is_empty() {
        return Err(format!("Tabela {table} não encontrada ou sem colunas."));
    }
    Ok(columns)
}

fn query_json(conn: &Connection, sql: &str) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("Falha ao preparar consulta: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok(Value::Object(row_to_json_map(row)?)))
        .map_err(|err| format!("Falha ao executar consulta: {err}"))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|err| format!("Falha ao mapear linha: {err}"))?);
    }
    Ok(result)
}

fn query_json_params<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Value>, String>
where
    P: rusqlite::Params,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("Falha ao preparar consulta: {err}"))?;
    let rows = stmt
        .query_map(params, |row| Ok(Value::Object(row_to_json_map(row)?)))
        .map_err(|err| format!("Falha ao executar consulta: {err}"))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|err| format!("Falha ao mapear linha: {err}"))?);
    }
    Ok(result)
}

fn query_one_json(conn: &Connection, sql: &str, id: i64) -> Result<Value, String> {
    conn.query_row(sql, [id], |row| Ok(Value::Object(row_to_json_map(row)?)))
        .optional()
        .map_err(|err| format!("Falha ao consultar registro: {err}"))?
        .ok_or_else(|| "Registro não encontrado.".to_string())
}

fn table_record(conn: &Connection, table: &str, id: i64) -> Result<Value, String> {
    let sql = format!("SELECT * FROM {} WHERE id = ?1", quote_ident(table));
    query_one_json(conn, &sql, id)
}

fn table_list(
    conn: &Connection,
    table: &str,
    search: &str,
    searchable: &[&str],
) -> Result<Vec<Value>, String> {
    let available = table_columns(conn, table)?;
    let order_col = if available.contains("nome") {
        "nome"
    } else if available.contains("descricao") {
        "descricao"
    } else if available.contains("id") {
        "id"
    } else {
        "rowid"
    };

    if search.trim().is_empty() {
        let sql = format!(
            "SELECT * FROM {} ORDER BY {}",
            quote_ident(table),
            quote_ident(order_col)
        );
        return query_json(conn, &sql);
    }

    let columns = searchable
        .iter()
        .copied()
        .filter(|field| available.contains(*field))
        .map(|field| format!("LOWER(COALESCE({}, '')) LIKE LOWER(?1)", quote_ident(field)))
        .collect::<Vec<_>>();

    if columns.is_empty() {
        let sql = format!(
            "SELECT * FROM {} ORDER BY {}",
            quote_ident(table),
            quote_ident(order_col)
        );
        return query_json(conn, &sql);
    }

    let sql = format!(
        "SELECT * FROM {} WHERE {} ORDER BY {}",
        quote_ident(table),
        columns.join(" OR "),
        quote_ident(order_col)
    );
    let like = format!("%{}%", search.trim());
    query_json_params(conn, &sql, params![like])
}

fn save_record(
    conn: &Connection,
    table: &str,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let available = table_columns(conn, table)?;
    let mut data = payload.clone();
    let timestamp = now();
    let id = data
        .get("id")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.parse::<i64>().ok())
        })
        .filter(|value| *value > 0);

    if available.contains("updated_at") && !data.contains_key("updated_at") {
        data.insert("updated_at".to_string(), Value::String(timestamp.clone()));
    }
    if id.is_none() && available.contains("created_at") && !data.contains_key("created_at") {
        data.insert("created_at".to_string(), Value::String(timestamp.clone()));
    }

    let mut fields = data
        .iter()
        .filter_map(|(key, value)| {
            if key == "id" || !available.contains(key) {
                None
            } else {
                Some((key.clone(), value.clone()))
            }
        })
        .collect::<Vec<_>>();
    fields.sort_by(|left, right| left.0.cmp(&right.0));

    if fields.is_empty() {
        return Err(format!("Nenhum campo válido foi informado para {table}."));
    }

    if let Some(id) = id {
        let assignments = fields
            .iter()
            .enumerate()
            .map(|(index, (field, _))| format!("{} = ?{}", quote_ident(field), index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let mut values = fields
            .iter()
            .map(|(_, value)| json_to_sql_value(value))
            .collect::<Vec<_>>();
        values.push(rusqlite::types::Value::Integer(id));
        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?{}",
            quote_ident(table),
            assignments,
            values.len()
        );
        conn.execute(&sql, params_from_iter(values.iter()))
            .map_err(|err| format!("Falha ao atualizar {table}: {err}"))?;
        table_record(conn, table, id)
    } else {
        let columns = fields
            .iter()
            .map(|(field, _)| quote_ident(field))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=fields.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let values = fields
            .iter()
            .map(|(_, value)| json_to_sql_value(value))
            .collect::<Vec<_>>();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_ident(table),
            columns,
            placeholders
        );
        conn.execute(&sql, params_from_iter(values.iter()))
            .map_err(|err| format!("Falha ao inserir em {table}: {err}"))?;
        table_record(conn, table, conn.last_insert_rowid())
    }
}

fn delete_record(conn: &Connection, table: &str, id: i64) -> Result<bool, String> {
    let sql = format!("DELETE FROM {} WHERE id = ?1", quote_ident(table));
    conn.execute(&sql, [id])
        .map_err(|err| format!("Falha ao excluir registro de {table}: {err}"))?;
    Ok(true)
}

fn as_i64_list(value: Option<&Value>) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_i64().or_else(|| item.as_str()?.parse::<i64>().ok()))
                .filter(|value| *value > 0)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn sync_link_table(
    conn: &Connection,
    table: &str,
    owner_field: &str,
    target_field: &str,
    owner_id: i64,
    target_ids: &[i64],
) -> Result<(), String> {
    conn.execute(
        &format!(
            "DELETE FROM {} WHERE {} = ?1",
            quote_ident(table),
            quote_ident(owner_field)
        ),
        [owner_id],
    )
    .map_err(|err| format!("Falha ao limpar vínculos de {table}: {err}"))?;
    let timestamp = now();
    for target_id in target_ids {
        conn.execute(
            &format!(
                "INSERT INTO {} ({}, {}, created_at) VALUES (?1, ?2, ?3)",
                quote_ident(table),
                quote_ident(owner_field),
                quote_ident(target_field)
            ),
            params![owner_id, target_id, timestamp],
        )
        .map_err(|err| format!("Falha ao gravar vínculo de {table}: {err}"))?;
    }
    Ok(())
}

fn profile_permissions(conn: &Connection, profile_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT permissao_chave FROM perfis_permissoes WHERE perfil_id = ?1 ORDER BY permissao_chave")
        .map_err(|err| format!("Falha ao preparar permissões: {err}"))?;
    let rows = stmt
        .query_map([profile_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("Falha ao consultar permissões: {err}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|err| format!("Falha ao mapear permissão: {err}"))?);
    }
    Ok(out)
}

fn user_record_for_api(conn: &Connection, user_id: i64) -> Result<Value, String> {
    let mut row = table_record(conn, "usuarios", user_id)?;
    let profile_ids = query_json_params(
        conn,
        "SELECT perfil_id AS id FROM usuarios_perfis WHERE usuario_id = ?1 ORDER BY perfil_id",
        [user_id],
    )?
    .into_iter()
    .filter_map(|item| item.get("id").and_then(Value::as_i64))
    .map(Value::from)
    .collect::<Vec<_>>();
    let company_ids = query_json_params(
        conn,
        "SELECT empresa_id AS id FROM usuarios_empresas WHERE usuario_id = ?1 ORDER BY empresa_id",
        [user_id],
    )?
    .into_iter()
    .filter_map(|item| item.get("id").and_then(Value::as_i64))
    .map(Value::from)
    .collect::<Vec<_>>();
    if let Value::Object(map) = &mut row {
        map.remove("senha_hash");
        map.remove("local_guest");
        map.insert("profile_ids".to_string(), Value::Array(profile_ids));
        map.insert("empresa_ids".to_string(), Value::Array(company_ids.clone()));
        map.insert("company_ids".to_string(), Value::Array(company_ids));
    }
    Ok(row)
}

fn save_settings_value(conn: &Connection, key: &str, value: &Value) -> Result<(), String> {
    let timestamp = now();
    conn.execute(
        "INSERT INTO app_settings (chave, valor, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor, updated_at = excluded.updated_at",
        params![key, value.to_string(), timestamp],
    )
    .map_err(|err| format!("Falha ao salvar configuração {key}: {err}"))?;
    Ok(())
}

fn load_settings_value(conn: &Connection, key: &str) -> Result<Option<Value>, String> {
    let raw = conn
        .query_row(
            "SELECT valor FROM app_settings WHERE chave = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar configuração {key}: {err}"))?;
    Ok(raw.and_then(|value| serde_json::from_str(&value).ok()))
}

fn runtime_settings_payload(state: &InternalApiState, settings: Value) -> Value {
    json!({
        "env_path": "app_settings:runtime.settings",
        "settings": settings,
        "ports": [
            {"key": "internal_api", "label": "API interna", "host": state.host, "configured_port": state.port, "effective_port": state.port, "default_port": 61001, "available": true, "fallback_applied": false},
            {"key": "local_web", "label": "Servidor web local", "host": "0.0.0.0", "configured_port": 61002, "effective_port": 61002, "default_port": 61002, "available": true, "fallback_applied": false},
            {"key": "auxiliary", "label": "Webhook Service", "host": "0.0.0.0", "configured_port": 61003, "effective_port": 61003, "default_port": 61003, "available": true, "fallback_applied": false},
            {"key": "bridge_core", "label": "WebSocket Service", "host": "0.0.0.0", "configured_port": 61004, "effective_port": 61004, "default_port": 61004, "available": true, "fallback_applied": false}
        ],
        "warnings": ["Configurações carregadas pela API interna web. Reinicie os serviços nativos pelo desktop quando necessário."]
    })
}

fn default_runtime_settings(state: &InternalApiState) -> Value {
    let mut settings = Map::new();

    settings.insert(
        "internal_api_host".to_string(),
        Value::String(state.host.clone()),
    );
    settings.insert("internal_api_port".to_string(), Value::from(state.port));
    settings.insert("internal_api_auto_start".to_string(), Value::Bool(true));
    settings.insert(
        "internal_api_base_url".to_string(),
        Value::String(format!("http://127.0.0.1:{}", state.port)),
    );
    settings.insert(
        "internal_api_docs_url".to_string(),
        Value::String(format!("http://127.0.0.1:{}/docs", state.port)),
    );
    settings.insert(
        "internal_api_restart_on_config_change".to_string(),
        Value::Bool(true),
    );
    settings.insert(
        "internal_api_require_token".to_string(),
        Value::Bool(state.config.security.require_token),
    );
    settings.insert(
        "internal_api_allow_public_network".to_string(),
        Value::Bool(state.config.security.allow_public_network),
    );
    settings.insert(
        "internal_api_cors_enabled".to_string(),
        Value::Bool(state.config.security.cors_enabled),
    );
    settings.insert(
        "internal_api_token_header".to_string(),
        Value::String(state.config.security.token_header.clone()),
    );
    settings.insert(
        "internal_api_token".to_string(),
        Value::String(String::new()),
    );
    settings.insert(
        "internal_api_docs_public_local".to_string(),
        Value::Bool(state.config.security.docs_public_local),
    );
    settings.insert(
        "internal_api_open_scalar_after_start".to_string(),
        Value::Bool(false),
    );
    settings.insert("internal_api_timeout_ms".to_string(), Value::from(8000));
    settings.insert(
        "internal_api_log_mode".to_string(),
        Value::String("normal".to_string()),
    );
    settings.insert(
        "internal_api_docs_enabled".to_string(),
        Value::Bool(state.config.expose_docs),
    );
    settings.insert(
        "internal_api_docs_path".to_string(),
        Value::String(state.config.docs.path.clone()),
    );
    settings.insert("local_web_enabled".to_string(), Value::Bool(true));
    settings.insert("local_web_auto_start".to_string(), Value::Bool(true));
    settings.insert(
        "local_web_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    settings.insert("local_web_port".to_string(), Value::from(61002));
    settings.insert(
        "auxiliary_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    settings.insert("auxiliary_port".to_string(), Value::from(61003));
    settings.insert(
        "bridge_core_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    settings.insert("bridge_core_port".to_string(), Value::from(61004));
    settings.insert("webhook_enabled".to_string(), Value::Bool(false));
    settings.insert("webhook_auto_start".to_string(), Value::Bool(false));
    settings.insert(
        "webhook_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    settings.insert("webhook_port".to_string(), Value::from(61003));
    settings.insert(
        "webhook_base_path".to_string(),
        Value::String("/webhooks".to_string()),
    );
    settings.insert("webhook_token_required".to_string(), Value::Bool(true));
    settings.insert(
        "webhook_token_header".to_string(),
        Value::String("X-Webhook-Token".to_string()),
    );
    settings.insert("webhook_token".to_string(), Value::String(String::new()));
    settings.insert("webhook_allow_lan".to_string(), Value::Bool(true));
    settings.insert("webhook_allow_external".to_string(), Value::Bool(false));
    settings.insert("websocket_enabled".to_string(), Value::Bool(false));
    settings.insert("websocket_auto_start".to_string(), Value::Bool(false));
    settings.insert(
        "websocket_host".to_string(),
        Value::String("0.0.0.0".to_string()),
    );
    settings.insert("websocket_port".to_string(), Value::from(61004));
    settings.insert(
        "websocket_path".to_string(),
        Value::String("/ws".to_string()),
    );
    settings.insert("websocket_token_required".to_string(), Value::Bool(true));
    settings.insert(
        "websocket_token_query".to_string(),
        Value::String("token".to_string()),
    );
    settings.insert(
        "websocket_token_header".to_string(),
        Value::String("X-WebSocket-Token".to_string()),
    );
    settings.insert("websocket_token".to_string(), Value::String(String::new()));
    settings.insert("websocket_allow_lan".to_string(), Value::Bool(true));
    settings.insert("websocket_allow_external".to_string(), Value::Bool(false));
    settings.insert("websocket_heartbeat_seconds".to_string(), Value::from(30));
    settings.insert("tray_enabled".to_string(), Value::Bool(false));
    settings.insert("minimize_to_tray".to_string(), Value::Bool(false));
    settings.insert("close_to_tray".to_string(), Value::Bool(false));
    settings.insert("start_with_windows".to_string(), Value::Bool(false));
    settings.insert("services_auto_start".to_string(), Value::Bool(true));
    settings.insert(
        "app_service_name".to_string(),
        Value::String("ManifestadorNFeServer".to_string()),
    );

    Value::Object(settings)
}

fn auth_login_http(
    state: &InternalApiState,
    login: String,
    senha: String,
    channel: String,
) -> Result<Value, String> {
    let channel = match channel.trim().to_ascii_lowercase().as_str() {
        "api" => "api",
        "mobile" | "remote-mobile" => "mobile",
        _ => {
            return Err(
                "Canal remoto inválido. Use api ou mobile para autenticação HTTP.".to_string(),
            )
        }
    };
    let conn = open_connection(&state.db_path)?;
    serde_json::to_value(login_with_channel(&conn, &login, &senha, channel)?)
        .map_err(|err| err.to_string())
}

fn auth_restore_http(state: &InternalApiState, session_token: String) -> Result<Value, String> {
    let conn = open_connection(&state.db_path)?;
    let identity = crate::commands::auth::load_session_identity(&conn, &session_token)?;
    if !matches!(identity.channel.as_str(), "api" | "mobile") {
        return Err("A sessão informada não pertence ao canal da API.".to_string());
    }
    let user = build_auth_user_for_channel(&conn, identity.user_id, Some(identity.channel))?;
    serde_json::to_value(crate::models::LoginResponse {
        success: true,
        message: "Sessão restaurada.".to_string(),
        session_token: Some(session_token),
        user: Some(user),
    })
    .map_err(|err| err.to_string())
}

fn auth_change_password_http(
    state: &InternalApiState,
    args: &Map<String, Value>,
) -> Result<Value, String> {
    let conn = open_connection(&state.db_path)?;
    change_password_with_connection(
        &conn,
        &state.db_path,
        &get_string(args, "session_token"),
        &get_string(args, "current_password"),
        &get_string(args, "new_password"),
    )
    .map(Value::Bool)
}

#[derive(Debug, Deserialize)]
pub struct AuthLoginRequest {
    pub login: String,
    pub senha: String,
    #[serde(default = "default_api_channel")]
    pub channel: String,
}

fn default_api_channel() -> String {
    "api".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AuthSessionRequest {
    pub session_token: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthChangePasswordRequest {
    pub session_token: String,
    pub current_password: String,
    pub new_password: String,
}

fn endpoint_response(
    result: Result<Value, String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match result {
        Ok(value) => Ok(Json(value)),
        Err(error) => Err((StatusCode::UNAUTHORIZED, Json(json!({ "error": error })))),
    }
}

pub async fn auth_login_endpoint(
    State(state): State<InternalApiState>,
    Json(request): Json<AuthLoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    endpoint_response(auth_login_http(
        &state,
        request.login,
        request.senha,
        request.channel,
    ))
}

pub async fn auth_restore_endpoint(
    State(state): State<InternalApiState>,
    Json(request): Json<AuthSessionRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    endpoint_response(auth_restore_http(&state, request.session_token))
}

pub async fn auth_logout_endpoint(
    State(state): State<InternalApiState>,
    Json(request): Json<AuthSessionRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    endpoint_response(open_connection(&state.db_path).and_then(|conn| {
        conn.execute(
            "DELETE FROM user_sessions WHERE session_token = ?1",
            [request.session_token],
        )
        .map_err(|err| format!("Falha ao encerrar sessão: {err}"))?;
        Ok(Value::Bool(true))
    }))
}

pub async fn auth_change_password_endpoint(
    State(state): State<InternalApiState>,
    Json(request): Json<AuthChangePasswordRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let args = Map::from_iter([
        (
            "session_token".to_string(),
            Value::String(request.session_token),
        ),
        (
            "current_password".to_string(),
            Value::String(request.current_password),
        ),
        (
            "new_password".to_string(),
            Value::String(request.new_password),
        ),
    ]);
    endpoint_response(auth_change_password_http(&state, &args))
}

pub async fn auth_password_policy_endpoint(
    State(state): State<InternalApiState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    endpoint_response(
        open_connection(&state.db_path)
            .and_then(|conn| load_password_policy(&conn).map(|policy| policy.to_public_value())),
    )
}

pub(crate) fn execute_command(
    state: &InternalApiState,
    command: &str,
    args: Map<String, Value>,
) -> Result<Value, String> {
    let public_or_first_access_command = matches!(
        command,
        "app_meta"
            | "app_bootstrap"
            | "system_info"
            | "auth_login"
            | "auth_restore"
            | "auth_logout"
            | "auth_change_password"
            | "auth_password_policy"
            | "auth_bootstrap_status"
    );
    let api_identity = if public_or_first_access_command {
        None
    } else {
        let conn = open_connection(&state.db_path)?;
        let identity = authorize_api_command(&conn, &get_string(&args, "session_token"), command)?;
        if command == "combo_list"
            && get_string(&args, "entity")
                .trim()
                .eq_ignore_ascii_case("empresas")
            && !has_permission(&conn, &identity, "empresas:view")?
        {
            return Err("Permissão necessária: empresas:view.".to_string());
        }
        Some(identity)
    };

    match command {
        "app_meta" => Ok(json!({
            "name": runtime_app_name(),
            "short_name": runtime_app_short_name(),
            "version": env!("CARGO_PKG_VERSION"),
            "identifier": runtime_app_identifier(),
            "runtime": "internal-api-web",
        })),
        "app_bootstrap" => Ok(json!({
            "ok": true,
            "runtime": {"mode": "internal-api-web", "isWeb": true, "isTauri": false},
            "database": "internal-api"
        })),
        "system_info" => Ok(json!({
            "runtime": {"mode": "internal-api-web"},
            "database_provider": std::env::var("MANIFESTADOR_NFE_DATABASE_DRIVER").unwrap_or_else(|_| "sqlite".to_string()),
            "online": true,
            "api": {"host": state.host, "port": state.port},
        })),
        "system_set_data_dir" => Ok(json!({
            "ok": false,
            "warning": "Alteração de diretório de dados deve ser aplicada pelo runtime desktop/Tauri."
        })),
        "auth_login" => {
            let login = if !get_string(&args, "login").is_empty() {
                get_string(&args, "login")
            } else {
                get_string(&args, "username")
            };
            let senha = if !get_string(&args, "senha").is_empty() {
                get_string(&args, "senha")
            } else {
                get_string(&args, "password")
            };
            let channel = get_string(&args, "channel");
            auth_login_http(
                state,
                login,
                senha,
                if channel.is_empty() {
                    "api".to_string()
                } else {
                    channel
                },
            )
        }
        "auth_restore" => auth_restore_http(state, get_string(&args, "session_token")),
        "auth_logout" => {
            let token = get_string(&args, "session_token");
            let conn = open_connection(&state.db_path)?;
            conn.execute(
                "DELETE FROM user_sessions WHERE session_token = ?1",
                [token],
            )
            .map_err(|err| format!("Falha ao encerrar sessão: {err}"))?;
            Ok(Value::Bool(true))
        }
        "auth_change_password" => auth_change_password_http(state, &args),
        "auth_password_policy" => {
            let conn = open_connection(&state.db_path)?;
            Ok(load_password_policy(&conn)?.to_public_value())
        }
        "auth_bootstrap_status" => {
            let conn = open_connection(&state.db_path)?;
            bootstrap_status_value(&conn)
        }
        "permission_catalog" => {
            let rows = crate::commands::auth::PERMISSION_CATALOG
                .iter()
                .map(|(key, label)| {
                    json!({
                        "key": key,
                        "label": label,
                        "descricao": label,
                        "module": key.split(':').next().unwrap_or("geral"),
                        "group": key.split(':').next().unwrap_or("geral")
                    })
                })
                .collect::<Vec<_>>();
            Ok(Value::Array(rows))
        }
        "profile_list" => {
            let conn = open_connection(&state.db_path)?;
            let mut rows = table_list(
                &conn,
                "perfis_acesso",
                &search_from_args(&args),
                &["nome", "descricao"],
            )?;
            for row in rows.iter_mut() {
                if let Value::Object(map) = row {
                    if let Some(id) = map.get("id").and_then(Value::as_i64) {
                        map.insert(
                            "permissions".to_string(),
                            Value::Array(
                                profile_permissions(&conn, id)?
                                    .into_iter()
                                    .map(Value::String)
                                    .collect(),
                            ),
                        );
                    }
                }
            }
            Ok(Value::Array(rows))
        }
        "profile_get" => {
            let conn = open_connection(&state.db_path)?;
            let mut row = table_record(&conn, "perfis_acesso", get_i64(&args, "id"))?;
            if let Value::Object(map) = &mut row {
                let id = map.get("id").and_then(Value::as_i64).unwrap_or_default();
                let permissions = profile_permissions(&conn, id)?;
                map.insert(
                    "permissions".to_string(),
                    Value::Array(permissions.iter().cloned().map(Value::String).collect()),
                );
                map.insert(
                    "permission_keys".to_string(),
                    Value::Array(permissions.into_iter().map(Value::String).collect()),
                );
            }
            Ok(row)
        }
        "profile_save" => {
            let conn = open_connection(&state.db_path)?;
            let mut payload = get_payload(&args);
            let requested_id = payload
                .get("id")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let name = payload
                .get("nome")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if name.is_empty() {
                return Err("Informe o nome do perfil.".to_string());
            }
            payload.insert("nome".to_string(), Value::String(name.to_string()));

            let permissions_supplied = ["permission_keys", "permissions", "permissoes"]
                .iter()
                .any(|key| payload.contains_key(*key));
            let mut permissions = payload
                .get("permission_keys")
                .or_else(|| payload.get("permissions"))
                .or_else(|| payload.get("permissoes"))
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            permissions.sort();
            permissions.dedup();
            for key in ["permission_keys", "permissions", "permissoes"] {
                payload.remove(key);
            }

            let current_master = if requested_id > 0 {
                conn.query_row(
                    "SELECT COALESCE(perfil_master, 0) FROM perfis_acesso WHERE id = ?1 LIMIT 1",
                    [requested_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao validar perfil: {err}"))?
                .ok_or_else(|| "Perfil não encontrado.".to_string())?
                    == 1
            } else {
                false
            };
            let profile_master = get_bool_value(&payload, "perfil_master", current_master);
            if profile_master {
                permissions = all_permission_keys();
            } else if (requested_id <= 0 || permissions_supplied) && permissions.is_empty() {
                return Err("Selecione ao menos uma permissão para o perfil.".to_string());
            }
            let valid_permissions = all_permission_keys();
            for permission in &permissions {
                if !valid_permissions.contains(permission) {
                    return Err(format!("Permissão inválida: {permission}."));
                }
            }

            let saved = save_record(&conn, "perfis_acesso", &payload)?;
            let saved_id = saved.get("id").and_then(Value::as_i64).unwrap_or_default();
            if profile_master || permissions_supplied || requested_id <= 0 {
                conn.execute(
                    "DELETE FROM perfis_permissoes WHERE perfil_id = ?1",
                    [saved_id],
                )
                .map_err(|err| format!("Falha ao limpar permissões: {err}"))?;
                for permission in permissions {
                    conn.execute(
                        "INSERT INTO perfis_permissoes (perfil_id, permissao_chave, created_at) VALUES (?1, ?2, ?3)",
                        params![saved_id, permission, now()],
                    )
                    .map_err(|err| format!("Falha ao gravar permissão: {err}"))?;
                }
            }
            let mut row = table_record(&conn, "perfis_acesso", saved_id)?;
            if let Value::Object(map) = &mut row {
                let current = profile_permissions(&conn, saved_id)?;
                map.insert(
                    "permission_keys".to_string(),
                    Value::Array(current.into_iter().map(Value::String).collect()),
                );
            }
            Ok(row)
        }
        "profile_delete" => {
            let conn = open_connection(&state.db_path)?;
            let id = get_i64(&args, "id");
            let linked_users = conn
                .query_row(
                    "SELECT COUNT(*) FROM usuarios_perfis WHERE perfil_id = ?1",
                    [id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|err| format!("Falha ao validar vínculos do perfil: {err}"))?;
            if linked_users > 0 {
                return Err(
                    "Não é possível excluir o perfil porque existem usuários vinculados a ele."
                        .to_string(),
                );
            }
            delete_record(&conn, "perfis_acesso", id).map(Value::Bool)
        }
        "user_policy_get" => {
            let conn = open_connection(&state.db_path)?;
            let login_min_length = load_login_min_length(&conn)?;
            let policy = load_password_policy(&conn)?;
            Ok(json!({
                "login_min_length": login_min_length,
                "login_min_allowed": USER_LOGIN_MIN_LENGTH_MIN,
                "login_max_allowed": USER_LOGIN_MIN_LENGTH_MAX,
                "password_min_length": policy.min_length,
                "password_max_length": policy.max_length,
                "password_min_allowed": crate::security::PASSWORD_MIN_ALLOWED,
                "password_max_allowed": crate::security::PASSWORD_MAX_ALLOWED,
                "password_require_letter": policy.require_letter,
                "password_require_number": policy.require_number,
                "password_require_uppercase": policy.require_uppercase,
                "password_require_lowercase": policy.require_lowercase,
                "password_require_symbol": policy.require_symbol
            }))
        }
        "user_policy_save" => {
            let conn = open_connection(&state.db_path)?;
            let payload = get_payload(&args);
            let login_min_length = save_login_min_length(
                &conn,
                payload
                    .get("login_min_length")
                    .and_then(|value| {
                        value
                            .as_i64()
                            .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
                    })
                    .unwrap_or(load_login_min_length(&conn)?),
            )?;
            let current = load_password_policy(&conn)?;
            let policy = save_password_policy(
                &conn,
                PasswordPolicy {
                    min_length: payload
                        .get("password_min_length")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize)
                        .unwrap_or(current.min_length),
                    max_length: payload
                        .get("password_max_length")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize)
                        .unwrap_or(current.max_length),
                    require_letter: get_bool_value(
                        &payload,
                        "password_require_letter",
                        current.require_letter,
                    ),
                    require_number: get_bool_value(
                        &payload,
                        "password_require_number",
                        current.require_number,
                    ),
                    require_uppercase: get_bool_value(
                        &payload,
                        "password_require_uppercase",
                        current.require_uppercase,
                    ),
                    require_lowercase: get_bool_value(
                        &payload,
                        "password_require_lowercase",
                        current.require_lowercase,
                    ),
                    require_symbol: get_bool_value(
                        &payload,
                        "password_require_symbol",
                        current.require_symbol,
                    ),
                },
            )?;
            Ok(json!({
                "login_min_length": login_min_length,
                "login_min_allowed": USER_LOGIN_MIN_LENGTH_MIN,
                "login_max_allowed": USER_LOGIN_MIN_LENGTH_MAX,
                "password_min_length": policy.min_length,
                "password_max_length": policy.max_length,
                "password_min_allowed": crate::security::PASSWORD_MIN_ALLOWED,
                "password_max_allowed": crate::security::PASSWORD_MAX_ALLOWED,
                "password_require_letter": policy.require_letter,
                "password_require_number": policy.require_number,
                "password_require_uppercase": policy.require_uppercase,
                "password_require_lowercase": policy.require_lowercase,
                "password_require_symbol": policy.require_symbol
            }))
        }
        "user_list" => {
            let conn = open_connection(&state.db_path)?;
            let search = search_from_args(&args);
            let rows = if search.is_empty() {
                query_json(
                    &conn,
                    "SELECT id, nome, login, email, telefone, cargo, photo_url, master_user, administrador, senha_provisoria, api_access_enabled, mobile_access_enabled, api_session_timeout_minutes, ativo, ultimo_login_em, created_at, updated_at FROM usuarios WHERE COALESCE(local_guest, 0) = 0 ORDER BY nome",
                )?
            } else {
                query_json_params(
                    &conn,
                    "SELECT id, nome, login, email, telefone, cargo, photo_url, master_user, administrador, senha_provisoria, api_access_enabled, mobile_access_enabled, api_session_timeout_minutes, ativo, ultimo_login_em, created_at, updated_at FROM usuarios WHERE COALESCE(local_guest, 0) = 0 AND (LOWER(nome) LIKE LOWER(?1) OR LOWER(login) LIKE LOWER(?1) OR LOWER(COALESCE(email, '')) LIKE LOWER(?1)) ORDER BY nome",
                    params![format!("%{}%", search)],
                )?
            };
            Ok(Value::Array(rows))
        }
        "user_get" => {
            let conn = open_connection(&state.db_path)?;
            let id = get_i64(&args, "id");
            let local_guest: i64 = conn
                .query_row(
                    "SELECT COALESCE(local_guest, 0) FROM usuarios WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao consultar usuário: {err}"))?
                .ok_or_else(|| "Usuário não encontrado.".to_string())?;
            if local_guest == 1 {
                return Err("O usuário técnico local não pode ser administrado.".to_string());
            }
            user_record_for_api(&conn, id)
        }
        "user_save" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let mut payload = get_payload(&args);
            let id = payload
                .get("id")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let profile_links_supplied = ["profile_ids", "perfil_ids", "perfis"]
                .iter()
                .any(|key| payload.contains_key(*key));
            let company_links_supplied = ["company_ids", "empresa_ids", "empresas"]
                .iter()
                .any(|key| payload.contains_key(*key));
            let profile_ids = as_i64_list(
                payload
                    .get("profile_ids")
                    .or_else(|| payload.get("perfil_ids"))
                    .or_else(|| payload.get("perfis")),
            );
            let company_ids = as_i64_list(
                payload
                    .get("company_ids")
                    .or_else(|| payload.get("empresa_ids"))
                    .or_else(|| payload.get("empresas")),
            );
            for key in [
                "profile_ids",
                "perfil_ids",
                "perfis",
                "company_ids",
                "empresa_ids",
                "empresas",
            ] {
                payload.remove(key);
            }

            let login = payload
                .get("login")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            let login_min_length = load_login_min_length(&conn)?;
            if login.chars().count() < login_min_length as usize {
                return Err(format!(
                    "O login deve conter ao menos {login_min_length} caracteres."
                ));
            }
            let name = payload
                .get("nome")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if name.is_empty() {
                return Err("Informe o nome do usuário.".to_string());
            }
            validate_optional_email(payload.get("email"))?;
            payload.insert("nome".to_string(), Value::String(name.to_string()));
            payload.insert("login".to_string(), Value::String(login.clone()));
            payload.remove("senha_hash");
            payload.remove("local_guest");
            payload.remove("session_channel");

            let existing = if id > 0 {
                Some(
                    conn.query_row(
                        "SELECT master_user, ativo, COALESCE(local_guest, 0) FROM usuarios WHERE id = ?1 LIMIT 1",
                        [id],
                        |row| {
                            Ok((
                                row.get::<_, i64>(0)? == 1,
                                row.get::<_, i64>(1)? == 1,
                                row.get::<_, i64>(2)? == 1,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|err| format!("Falha ao validar usuário: {err}"))?
                    .ok_or_else(|| "Usuário não encontrado.".to_string())?,
                )
            } else {
                None
            };
            if existing.is_some_and(|item| item.2) {
                return Err("O usuário técnico local não pode ser administrado.".to_string());
            }
            let master_user =
                get_bool_value(&payload, "master_user", existing.is_some_and(|item| item.0));
            let active = get_bool_value(&payload, "ativo", existing.is_none_or(|item| item.1));
            if id == identity.user_id && (!master_user || !active) {
                return Err(
                    "O usuário master da sessão não pode ser inativado nem perder o perfil master."
                        .to_string(),
                );
            }

            let duplicate = conn
                .query_row(
                    "SELECT id FROM usuarios WHERE LOWER(login) = LOWER(?1) AND (?2 <= 0 OR id <> ?2) LIMIT 1",
                    params![login, id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao validar login do usuário: {err}"))?;
            if duplicate.is_some() {
                return Err("Já existe usuário com este login.".to_string());
            }

            if !master_user {
                if (id <= 0 || profile_links_supplied) && profile_ids.is_empty() {
                    return Err(
                        "Selecione ao menos um perfil de acesso para o usuário.".to_string()
                    );
                }
                if (id <= 0 || company_links_supplied) && company_ids.is_empty() {
                    return Err("Selecione ao menos uma empresa para o usuário.".to_string());
                }
            }
            for profile_id in &profile_ids {
                let exists = conn
                    .query_row(
                        "SELECT 1 FROM perfis_acesso WHERE id = ?1 AND ativo = 1 LIMIT 1",
                        [profile_id],
                        |_| Ok(1_i64),
                    )
                    .optional()
                    .map_err(|err| format!("Falha ao validar perfil do usuário: {err}"))?;
                if exists.is_none() {
                    return Err(format!("Perfil inválido informado: {profile_id}."));
                }
            }
            for company_id in &company_ids {
                let exists = conn
                    .query_row(
                        "SELECT 1 FROM empresas WHERE id = ?1 AND ativo = 1 LIMIT 1",
                        [company_id],
                        |_| Ok(1_i64),
                    )
                    .optional()
                    .map_err(|err| format!("Falha ao validar empresa do usuário: {err}"))?;
                if exists.is_none() {
                    return Err(format!("Empresa inválida informada: {company_id}."));
                }
            }

            // Nunca aceita hash fornecido pelo cliente; somente o backend gera Argon2.
            payload.remove("senha_hash");
            let password = payload
                .get("senha")
                .or_else(|| payload.get("password"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if id <= 0 && password.is_empty() {
                return Err("Informe a senha inicial do novo usuário.".to_string());
            }
            if !password.is_empty() {
                load_password_policy(&conn)?.validate(&password)?;
                payload.insert(
                    "senha_hash".to_string(),
                    Value::String(hash_password(&password)?),
                );
                payload.insert("senha_provisoria".to_string(), json!(1));
            }
            payload.remove("senha");
            payload.remove("password");
            payload.insert("local_guest".to_string(), json!(0));
            payload.insert("master_user".to_string(), Value::Bool(master_user));
            payload.insert("ativo".to_string(), Value::Bool(active));
            payload
                .entry("api_access_enabled".to_string())
                .or_insert(json!(0));
            payload
                .entry("mobile_access_enabled".to_string())
                .or_insert(json!(1));
            let timeout = payload
                .get("api_session_timeout_minutes")
                .and_then(|value| {
                    value
                        .as_i64()
                        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
                })
                .unwrap_or(720)
                .clamp(15, 43_200);
            payload.insert(
                "api_session_timeout_minutes".to_string(),
                Value::from(timeout),
            );

            let saved = save_record(&conn, "usuarios", &payload)?;
            let saved_id = saved.get("id").and_then(Value::as_i64).unwrap_or_default();
            if profile_links_supplied || id <= 0 {
                sync_link_table(
                    &conn,
                    "usuarios_perfis",
                    "usuario_id",
                    "perfil_id",
                    saved_id,
                    &profile_ids,
                )?;
            }
            if company_links_supplied || id <= 0 {
                sync_link_table(
                    &conn,
                    "usuarios_empresas",
                    "usuario_id",
                    "empresa_id",
                    saved_id,
                    &company_ids,
                )?;
            }
            user_record_for_api(&conn, saved_id)
        }
        "user_delete" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let id = get_i64(&args, "id");
            if id == identity.user_id {
                return Err("O usuário master da sessão não pode excluir a si mesmo.".to_string());
            }
            let protected = conn
                .query_row(
                    "SELECT master_user, COALESCE(local_guest, 0) FROM usuarios WHERE id = ?1 LIMIT 1",
                    [id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|err| format!("Falha ao validar usuário: {err}"))?
                .ok_or_else(|| "Usuário não encontrado.".to_string())?;
            if protected.0 == 1 || protected.1 == 1 {
                return Err(
                    "Usuários master ou técnicos não podem ser excluídos pela API.".to_string(),
                );
            }
            delete_record(&conn, "usuarios", id).map(Value::Bool)
        }
        "company_list" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let search = search_from_args(&args);
            let rows = if identity.master_user {
                table_list(
                    &conn,
                    "empresas",
                    &search,
                    &["nome", "nome_fantasia", "documento", "cidade", "estado"],
                )?
            } else if search.is_empty() {
                query_json_params(
                    &conn,
                    "SELECT e.* FROM empresas e INNER JOIN usuarios_empresas ue ON ue.empresa_id = e.id WHERE ue.usuario_id = ?1 ORDER BY e.nome",
                    [identity.user_id],
                )?
            } else {
                query_json_params(
                    &conn,
                    "SELECT e.* FROM empresas e INNER JOIN usuarios_empresas ue ON ue.empresa_id = e.id WHERE ue.usuario_id = ?1 AND (LOWER(e.nome) LIKE LOWER(?2) OR LOWER(COALESCE(e.nome_fantasia, '')) LIKE LOWER(?2) OR LOWER(COALESCE(e.documento, '')) LIKE LOWER(?2)) ORDER BY e.nome",
                    params![identity.user_id, format!("%{}%", search)],
                )?
            };
            Ok(Value::Array(rows))
        }
        "company_get" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let id = get_i64(&args, "id");
            if !company_allowed(&conn, identity, id)? {
                return Err("Empresa fora do escopo do usuário.".to_string());
            }
            table_record(&conn, "empresas", id)
        }
        "company_save" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let payload = get_payload(&args);
            let requested_id = payload
                .get("id")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            if requested_id > 0 && !company_allowed(&conn, identity, requested_id)? {
                return Err("Empresa fora do escopo do usuário.".to_string());
            }
            let saved = save_record(&conn, "empresas", &payload)?;
            let saved_id = saved.get("id").and_then(Value::as_i64).unwrap_or_default();
            if !identity.master_user && requested_id <= 0 {
                conn.execute(
                    "INSERT OR IGNORE INTO usuarios_empresas (usuario_id, empresa_id, created_at) VALUES (?1, ?2, ?3)",
                    params![identity.user_id, saved_id, now()],
                )
                .map_err(|err| format!("Falha ao vincular empresa criada ao usuário: {err}"))?;
            }
            Ok(saved)
        }
        "company_delete" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let id = get_i64(&args, "id");
            if !company_allowed(&conn, identity, id)? {
                return Err("Empresa fora do escopo do usuário.".to_string());
            }
            delete_record(&conn, "empresas", id).map(Value::Bool)
        }
        "manifestador_summary" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let requested = args.get("empresa_id").and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
            });
            let company_id = resolve_manifestador_company(&conn, identity, requested)?;
            crate::manifestador::summary(&conn, company_id)
        }
        "manifestador_config_get" => {
            if get_i64(&args, "empresa_id") <= 0 {
                return Err("Informe a empresa dos parâmetros fiscais.".into());
            }
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity.as_ref().ok_or("Sessão não encontrada.")?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            crate::manifestador::get_config(&conn, company_id)
        }
        "manifestador_config_save" => {
            if get_i64(&args, "empresa_id") <= 0 {
                return Err("Informe a empresa dos parâmetros fiscais.".into());
            }
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity.as_ref().ok_or("Sessão não encontrada.")?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            crate::manifestador::save_config(&conn, company_id, &get_payload(&args))
        }
        "manifestador_company_config_get" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            crate::manifestador::get_company_config(&conn, company_id)
        }
        "manifestador_company_config_save" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            crate::manifestador::save_company_config(&conn, company_id, &get_payload(&args))
        }
        "manifestador_notes_list" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let mut filters = get_filters(&args);
            let requested = filters.get("empresa_id").and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
            });
            if let Some(company_id) = resolve_manifestador_company(&conn, identity, requested)? {
                filters.insert("empresa_id".to_string(), Value::from(company_id));
            }
            Ok(Value::Array(
                crate::manifestador::list_notes(&conn, &filters)?
                    .into_iter()
                    .map(Value::Object)
                    .collect(),
            ))
        }
        "manifestador_note_get" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let note_id = get_i64(&args, "nota_id");
            let valid: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
                    params![note_id, company_id],
                    |row| row.get(0),
                )
                .map_err(|err| format!("Falha ao validar NF-e: {err}"))?;
            if valid == 0 {
                return Err("NF-e não pertence à empresa ativa.".to_string());
            }
            crate::manifestador::note_json(&conn, note_id)
        }
        "manifestador_note_logs" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let note_id = get_i64(&args, "nota_id");
            let valid: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
                    params![note_id, company_id],
                    |row| row.get(0),
                )
                .map_err(|err| format!("Falha ao validar histórico: {err}"))?;
            if valid == 0 {
                return Err("NF-e não pertence à empresa ativa.".to_string());
            }
            Ok(Value::Array(
                crate::manifestador::list_logs(&conn, note_id)?
                    .into_iter()
                    .map(Value::Object)
                    .collect(),
            ))
        }
        "manifestador_import_xml" => {
            let mut conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            crate::manifestador::import_xml(
                &mut conn,
                company_id,
                &get_string(&args, "xml"),
                get_i64(&args, "nsu"),
                None,
            )
        }
        "manifestador_obter_notas" => {
            let mut conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let request = json!({
                "empresa_id": company_id,
                "data_inicial": args.get("data_inicial"),
                "data_final": args.get("data_final")
            });
            let (response, mode) = if let Some(response) =
                crate::manifestador::call_provider(&conn, "obter_notas", &request)?
            {
                (response, "provider")
            } else {
                (
                    crate::manifestador::native_obter_notas(&conn, company_id)?,
                    "native",
                )
            };
            let mut imported = 0i64;
            let documents = response
                .get("documentos")
                .or_else(|| response.get("notas"))
                .or_else(|| response.get("xmls"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for document in documents {
                if crate::manifestador::import_provider_document(&mut conn, company_id, &document)?
                {
                    imported += 1;
                }
            }
            Ok(json!({
                "ok": true,
                "mode": mode,
                "imported": imported,
                "message": format!("{imported} documento(s) recebido(s) pela integração {mode}.")
            }))
        }
        "manifestador_download" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let nota_id = get_i64(&args, "nota_id");
            if let Some(xml) = args
                .get("xml")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
            {
                return crate::manifestador::download_note(
                    &conn,
                    &state.data_dir,
                    company_id,
                    nota_id,
                    Some(xml),
                );
            }
            let chave: String = conn
                .query_row(
                    "SELECT chave FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
                    params![nota_id, company_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao localizar NF-e para download: {err}"))?
                .ok_or_else(|| "NF-e não pertence à empresa ativa.".to_string())?;
            let request = json!({"empresa_id": company_id, "nota_id": nota_id, "chave": chave});
            if let Some(response) = crate::manifestador::call_provider(&conn, "download", &request)?
            {
                let mut full_xml = response.get("xml").map(value_text).unwrap_or_default();
                if full_xml.is_empty() {
                    full_xml = response.get("docZip").map(value_text).unwrap_or_default();
                    if !full_xml.is_empty() {
                        full_xml = crate::manifestador::decode_doczip(&full_xml)?;
                    }
                }
                if full_xml.trim().is_empty() {
                    return Err(
                        "O adaptador externo não retornou o XML completo da NF-e.".to_string()
                    );
                }
                return crate::manifestador::download_note(
                    &conn,
                    &state.data_dir,
                    company_id,
                    nota_id,
                    Some(&full_xml),
                );
            }
            crate::manifestador::native_download_note(&conn, company_id, nota_id, &state.data_dir)
        }
        "manifestador_manifestar" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let note_id = get_i64(&args, "nota_id");
            let event = get_string(&args, "evento");
            let justification = get_string(&args, "justificativa");
            let chave: String = conn
                .query_row(
                    "SELECT chave FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
                    params![note_id, company_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao localizar NF-e para manifestação: {err}"))?
                .ok_or_else(|| "NF-e não pertence à empresa ativa.".to_string())?;
            let request = json!({
                "empresa_id": company_id,
                "nota_id": note_id,
                "chave": chave,
                "evento": event,
                "justificativa": justification
            });
            if let Some(response) =
                crate::manifestador::call_provider(&conn, "manifestar", &request)?
            {
                let protocol = response.get("protocolo").and_then(Value::as_str);
                let note = crate::manifestador::update_manifestation(
                    &conn,
                    note_id,
                    &event,
                    &justification,
                    protocol,
                )?;
                return Ok(json!({
                    "mode": "provider",
                    "response": response,
                    "note": note,
                    "message": "Manifestação enviada ao adaptador e registrada localmente."
                }));
            }
            let note = crate::manifestador::update_manifestation(
                &conn,
                note_id,
                &event,
                &justification,
                None,
            )?;
            Ok(json!({
                "mode": "local",
                "note": note,
                "warning": "Manifestação registrada localmente. Configure um adaptador externo para assinatura e envio real à SEFAZ."
            }))
        }
        "manifestador_export_zip" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let requested = args.get("empresa_id").and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
            });
            let company_id = resolve_manifestador_company(&conn, identity, requested)?;
            crate::manifestador::export_zip(&conn, &state.data_dir, company_id, &get_filters(&args))
        }
        "manifestador_send_accounting" => {
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity
                .as_ref()
                .ok_or_else(|| "Sessão não encontrada.".to_string())?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let data_inicial = get_string(&args, "data_inicial");
            let data_final = get_string(&args, "data_final");
            let recipient = get_string(&args, "destinatario");
            if data_inicial.is_empty() || data_final.is_empty() {
                return Err("Informe o período para envio à contabilidade.".to_string());
            }
            if recipient.is_empty() {
                return Err("Informe o e-mail destinatário da contabilidade.".to_string());
            }
            crate::manifestador::send_accounting_email(
                &conn,
                &state.data_dir,
                company_id,
                &data_inicial,
                &data_final,
                &recipient,
            )
        }
        "manifestador_provider_test" => {
            if get_i64(&args, "empresa_id") <= 0 {
                return Err("Informe a empresa dos parâmetros fiscais.".into());
            }
            let conn = open_connection(&state.db_path)?;
            let identity = api_identity.as_ref().ok_or("Sessão não encontrada.")?;
            let company_id =
                required_manifestador_company(&conn, identity, Some(get_i64(&args, "empresa_id")))?;
            let Some((endpoint, timeout)) =
                crate::manifestador::provider_endpoint(&conn, company_id)?
            else {
                return Ok(json!({
                    "configured": false,
                    "message": "Nenhum endpoint externo configurado."
                }));
            };
            Ok(json!({
                "configured": true,
                "endpoint": endpoint,
                "timeout_seconds": timeout,
                "message": "Endpoint configurado. O teste efetivo ocorre ao consultar notas ou enviar manifestação."
            }))
        }
        "company_lookup_cnpj" | "company_lookup_ie" => Ok(
            json!({"ok": false, "message": "Consulta externa não configurada nesta API interna."}),
        ),
        "entity_list" => {
            let provider = provider_for_state(state)?;
            let entity = get_string(&args, "entity");
            let search = search_from_args(&args);
            Ok(Value::Array(
                entities::provider_list_with_database(&provider, &entity, &search)?
                    .into_iter()
                    .map(Value::Object)
                    .collect(),
            ))
        }
        "entity_save" => {
            let provider = provider_for_state(state)?;
            Ok(Value::Object(entities::provider_save_with_database(
                &provider,
                get_string(&args, "entity"),
                get_payload(&args),
            )?))
        }
        "entity_delete" => {
            let provider = provider_for_state(state)?;
            Ok(Value::Bool(entities::provider_delete_with_database(
                &provider,
                get_string(&args, "entity"),
                get_i64(&args, "id"),
            )?))
        }
        "combo_list" => {
            let entity = get_string(&args, "entity");
            let rows = if entity.eq_ignore_ascii_case("empresas") {
                let conn = open_connection(&state.db_path)?;
                let identity = api_identity
                    .as_ref()
                    .ok_or_else(|| "Sessão não encontrada.".to_string())?;
                if identity.master_user {
                    query_json(&conn, "SELECT id, nome FROM empresas ORDER BY nome")?
                } else {
                    query_json_params(
                        &conn,
                        "SELECT e.id, e.nome FROM empresas e INNER JOIN usuarios_empresas ue ON ue.empresa_id = e.id WHERE ue.usuario_id = ?1 ORDER BY e.nome",
                        [identity.user_id],
                    )?
                }
                .into_iter()
                .filter_map(|value| value.as_object().cloned())
                .collect::<Vec<_>>()
            } else {
                let provider = provider_for_state(state)?;
                entities::provider_list_with_database(&provider, &entity, "")?
            };
            let result = rows
                .into_iter()
                .map(|row| {
                    let id = row.get("id").cloned().unwrap_or(Value::Null);
                    let label = row
                        .get("nome")
                        .or_else(|| row.get("descricao"))
                        .or_else(|| row.get("titulo"))
                        .cloned()
                        .unwrap_or_else(|| Value::String(format!("Registro {}", id)));
                    json!({ "id": id, "label": label })
                })
                .collect::<Vec<_>>();
            Ok(Value::Array(result))
        }
        "app_log_write" => {
            let payload = get_payload(&args);
            let conn = open_connection(&state.db_path)?;
            let details = payload.get("details").cloned().unwrap_or(Value::Null);
            write_app_log(
                &conn,
                &state.data_dir,
                AppLogInput {
                    level: payload
                        .get("level")
                        .and_then(Value::as_str)
                        .unwrap_or("info"),
                    category: payload
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or("frontend"),
                    message: payload
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Evento da aplicação web"),
                    source: payload.get("source").and_then(Value::as_str),
                    route: payload.get("route").and_then(Value::as_str),
                    details: Some(&details),
                },
            )?;
            Ok(Value::Bool(true))
        }
        "app_log_list" => {
            let conn = open_connection(&state.db_path)?;
            Ok(Value::Array(query_json(&conn, "SELECT id, level, category, message, source, route, details_json, created_at FROM app_logs ORDER BY id DESC LIMIT 300")?))
        }
        "app_log_clear" => {
            let conn = open_connection(&state.db_path)?;
            conn.execute("DELETE FROM app_logs", [])
                .map_err(|err| format!("Falha ao limpar logs: {err}"))?;
            Ok(Value::Bool(true))
        }
        "licensing_load_settings"
        | "licensing_status"
        | "licensing_device_info"
        | "licensing_check_runtime"
        | "support_guard_status" => Ok(json!({
            "enabled": false,
            "status": "disabled",
            "message": "Recurso administrativo disponível pelo runtime desktop/Tauri."
        })),
        "licensing_save_settings"
        | "licensing_start_trial"
        | "support_guard_provision"
        | "support_guard_enable_totp"
        | "support_guard_unlock" => Ok(json!({
            "ok": true,
            "message": "Solicitação registrada pela API interna web."
        })),
        "integration_list" => {
            let conn = open_connection(&state.db_path)?;
            Ok(Value::Array(table_list(
                &conn,
                "integration_configs",
                &search_from_args(&args),
                &["nome", "tipo", "base_url", "ambiente", "status"],
            )?))
        }
        "integration_save" => {
            let conn = open_connection(&state.db_path)?;
            save_record(&conn, "integration_configs", &get_payload(&args))
        }
        "integration_delete" => {
            let conn = open_connection(&state.db_path)?;
            delete_record(&conn, "integration_configs", get_i64(&args, "id")).map(Value::Bool)
        }
        "integration_test" => Ok(
            json!({"ok": true, "status": "not_tested", "message": "Teste real de integração disponível no desktop/API dedicada."}),
        ),
        "integration_logs" => {
            let conn = open_connection(&state.db_path)?;
            Ok(Value::Array(query_json(
                &conn,
                "SELECT * FROM integration_logs ORDER BY id DESC LIMIT 200",
            )?))
        }
        "sync_queue_list" => {
            let conn = open_connection(&state.db_path)?;
            Ok(Value::Array(query_json(&conn, "SELECT id, entity_name, action_name, record_id, payload_json, status, created_at, updated_at FROM sync_queue ORDER BY id DESC LIMIT 300")?))
        }
        "sync_queue_mark_synced" => {
            let conn = open_connection(&state.db_path)?;
            conn.execute(
                "UPDATE sync_queue SET status = 'synced', updated_at = ?1 WHERE id = ?2",
                params![now(), get_i64(&args, "id")],
            )
            .map_err(|err| format!("Falha ao marcar item como sincronizado: {err}"))?;
            Ok(Value::Bool(true))
        }
        "runtime_settings_load" => {
            let conn = open_connection(&state.db_path)?;
            let settings = load_settings_value(&conn, "runtime.settings")?
                .unwrap_or_else(|| default_runtime_settings(state));
            Ok(runtime_settings_payload(state, settings))
        }
        "runtime_settings_save" => {
            let conn = open_connection(&state.db_path)?;
            let settings = args
                .get("settings")
                .cloned()
                .unwrap_or_else(|| Value::Object(get_payload(&args)));
            save_settings_value(&conn, "runtime.settings", &settings)?;
            Ok(runtime_settings_payload(state, settings))
        }
        "runtime_env_example" => Ok(Value::String(crate::runtime_config::default_env_template())),
        "internal_api_status" => {
            let web_status = crate::internal_api::web_server::status().ok();
            Ok(json!({
                "running": true,
                "status": "running",
                "host": state.host,
                "port": state.port,
                "url": format!("http://127.0.0.1:{}", state.port),
                "bind_url": format!("http://{}:{}", state.host, state.port),
                "docs_url": format!("http://127.0.0.1:{}/docs", state.port),
                "token_required": state.config.security.require_token,
                "auto_start": true,
                "web_server": web_status
            }))
        }
        "internal_api_test" => {
            Ok(json!({"ok": true, "url": format!("http://127.0.0.1:{}/health", state.port)}))
        }
        "web_proxy_status" => Ok(serde_json::to_value(
            crate::internal_api::web_server::status().ok(),
        )
        .unwrap_or(Value::Null)),
        "web_proxy_start" | "web_proxy_restart" => Ok(
            json!({"ok": true, "running": true, "message": "Webport já está ativo, pois esta chamada chegou pela própria API interna/proxy."}),
        ),
        "web_proxy_stop" => Ok(
            json!({"ok": false, "running": true, "warning": "O webport que serve a sessão atual não pode ser parado pela própria sessão web."}),
        ),
        "internal_api_test_port" => {
            Ok(json!({"available": false, "running_here": true, "port": state.port}))
        }
        "internal_api_start" | "internal_api_restart" => {
            Ok(json!({"ok": true, "running": true, "message": "API já está ativa neste processo."}))
        }
        "internal_api_stop" => Ok(
            json!({"ok": false, "running": true, "warning": "A API que serve a aplicação web não pode ser parada por esta própria sessão."}),
        ),
        "webhook_status" => Ok(
            serde_json::to_value(crate::native_webhook::server::status()?)
                .map_err(|err| format!("Falha ao serializar status do Webhook Service: {err}"))?,
        ),
        "webhook_start" | "webhook_restart" => Ok(
            json!({"running": crate::native_webhook::server::status()?.running, "runtime": "internal-api-web", "message": "Inicie/reinicie o Webhook Service pelo desktop/headless; a sessão web apenas monitora o serviço real."}),
        ),
        "webhook_stop" => Ok(serde_json::to_value(
            crate::native_webhook::server::stop_background()?
        )
        .map_err(|err| format!("Falha ao serializar stop do Webhook Service: {err}"))?),
        "webhook_list_events" => {
            Ok(json!({"items": crate::native_webhook::routes::list_events_snapshot()}))
        }
        "webhook_clear_events" => {
            crate::native_webhook::routes::clear_events();
            Ok(Value::Bool(true))
        }
        "webhook_test_receive" => Ok(
            json!({"ok": true, "event": {"provider": get_string(&args, "provider"), "event": get_string(&args, "event"), "status": "received", "received_at": now()}}),
        ),
        "websocket_status" => Ok(
            serde_json::to_value(crate::native_websocket::server::status()?)
                .map_err(|err| format!("Falha ao serializar status do WebSocket Service: {err}"))?,
        ),
        "websocket_start" | "websocket_restart" => Ok(
            json!({"running": crate::native_websocket::server::status()?.running, "runtime": "internal-api-web", "message": "Inicie/reinicie o WebSocket Service pelo desktop/headless; a sessão web apenas monitora o serviço real."}),
        ),
        "websocket_stop" => Ok(serde_json::to_value(
            crate::native_websocket::server::stop_background()?,
        )
        .map_err(|err| format!("Falha ao serializar stop do WebSocket Service: {err}"))?),
        "websocket_list_clients" => {
            Ok(json!({"items": crate::native_websocket::hub::list_clients()}))
        }
        "websocket_broadcast_test" => {
            let payload = json!({"type": "template.broadcast.test", "message": get_string(&args, "message"), "sent_at": now()});
            let sent = crate::native_websocket::hub::broadcast_text(payload.to_string());
            Ok(json!({"ok": true, "sent": sent, "payload": payload}))
        }
        "startup_with_windows_set" => Ok(
            json!({"implemented": false, "runtime": "internal-api-web", "message": "Inicialização com Windows deve ser aplicada pelo runtime desktop/Tauri."}),
        ),
        "app_service_install"
        | "app_service_uninstall"
        | "app_service_start"
        | "app_service_stop"
        | "app_service_restart" => Ok(
            json!({"ok": false, "warning": "Controle de serviço nativo indisponível pela sessão web interna."}),
        ),
        "app_service_status" => Ok(
            json!({"running": false, "installed": false, "available": false, "runtime": "internal-api-web"}),
        ),
        "tray_status" => Ok(
            json!({"enabled": false, "available": false, "runtime": "internal-api-web", "message": "Tray icon é recurso do processo desktop."}),
        ),
        "open_print_preview" => Ok(Value::Bool(true)),
        "tray_restore_window" | "tray_exit_app" => Ok(Value::Bool(true)),
        "provider_list_with_database" => Ok(json!([
            {"id": "sqlite", "name": "SQLite interno", "runtime": "internal-api-web"},
            {"id": "mysql", "name": "MySQL", "runtime": "internal-api-web"},
            {"id": "postgres", "name": "PostgreSQL", "runtime": "internal-api-web"}
        ])),
        "entity_provider_list" | "provider_entity_list" | "entity_provider_list_data" => {
            let provider = provider_for_state(state)?;
            let entity = get_string(&args, "entity");
            Ok(Value::Array(
                entities::provider_list_with_database(
                    &provider,
                    &entity,
                    &search_from_args(&args),
                )?
                .into_iter()
                .map(Value::Object)
                .collect(),
            ))
        }
        "provider_entity_get" => {
            let provider = provider_for_state(state)?;
            Ok(Value::Object(
                entities::provider_get_with_database(
                    &provider,
                    &get_string(&args, "entity"),
                    get_i64(&args, "id"),
                )?
                .ok_or_else(|| "Registro não encontrado.".to_string())?,
            ))
        }
        "provider_entity_create" | "provider_entity_update" => {
            let provider = provider_for_state(state)?;
            Ok(Value::Object(entities::provider_save_with_database(
                &provider,
                get_string(&args, "entity"),
                get_payload(&args),
            )?))
        }
        "provider_entity_delete" => {
            let provider = provider_for_state(state)?;
            Ok(Value::Bool(entities::provider_delete_with_database(
                &provider,
                get_string(&args, "entity"),
                get_i64(&args, "id"),
            )?))
        }
        other if other.ends_with("_list") => {
            let entity = other.trim_end_matches("_list").replace('_', "-");
            let provider = provider_for_state(state)?;
            Ok(Value::Array(
                entities::provider_list_with_database(
                    &provider,
                    &entity,
                    &search_from_args(&args),
                )?
                .into_iter()
                .map(Value::Object)
                .collect(),
            ))
        }
        other if other.ends_with("_get") => {
            let entity = other.trim_end_matches("_get").replace('_', "-");
            let provider = provider_for_state(state)?;
            Ok(Value::Object(
                entities::provider_get_with_database(&provider, &entity, get_i64(&args, "id"))?
                    .ok_or_else(|| "Registro não encontrado.".to_string())?,
            ))
        }
        other if other.ends_with("_save") => {
            let entity = other.trim_end_matches("_save").replace('_', "-");
            let provider = provider_for_state(state)?;
            Ok(Value::Object(entities::provider_save_with_database(
                &provider,
                entity,
                get_payload(&args),
            )?))
        }
        other if other.ends_with("_delete") => {
            let entity = other.trim_end_matches("_delete").replace('_', "-");
            let provider = provider_for_state(state)?;
            Ok(Value::Bool(entities::provider_delete_with_database(
                &provider,
                entity,
                get_i64(&args, "id"),
            )?))
        }
        _ => Err(format!(
            "Comando ainda não exposto na API web interna: {command}"
        )),
    }
}
