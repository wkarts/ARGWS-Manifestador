use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use tauri::State;

use crate::{app_state::SharedState, db::open_connection, manifestador, models::SessionIdentity};

fn session_and_connection(
    state: &State<'_, SharedState>,
    session_token: &str,
    permission: &str,
) -> Result<(Connection, SessionIdentity), String> {
    let db_path = state.db_path()?;
    let conn = open_connection(&db_path)?;
    let identity = crate::commands::auth::require_session_by_token(&conn, session_token)?;
    if !identity.master_user {
        let allowed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usuarios_perfis up INNER JOIN perfis_permissoes pp ON pp.perfil_id = up.perfil_id WHERE up.usuario_id = ?1 AND pp.permissao_chave = ?2",
                params![identity.user_id, permission],
                |row| row.get(0),
            )
            .map_err(|err| format!("Falha ao verificar permissão do Manifestador: {err}"))?;
        if allowed == 0 {
            return Err("Usuário sem permissão para acessar o Manifestador de NF-e.".to_string());
        }
    }
    Ok((conn, identity))
}

fn resolve_company(
    conn: &Connection,
    identity: &SessionIdentity,
    requested: Option<i64>,
) -> Result<Option<i64>, String> {
    if let Some(company_id) = requested.filter(|value| *value > 0) {
        if !identity.master_user {
            let linked: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM usuarios_empresas WHERE usuario_id = ?1 AND empresa_id = ?2",
                    params![identity.user_id, company_id],
                    |row| row.get(0),
                )
                .map_err(|err| format!("Falha ao validar empresa ativa: {err}"))?;
            if linked == 0 {
                return Err("Usuário sem vínculo com a empresa selecionada.".to_string());
            }
        }
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM empresas WHERE id = ?1 AND ativo = 1",
                [company_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("Falha ao verificar empresa ativa: {err}"))?;
        if exists == 0 {
            return Err("Empresa ativa não encontrada ou inativa.".to_string());
        }
        return Ok(Some(company_id));
    }

    if identity.master_user {
        return Ok(None);
    }
    conn.query_row(
        "SELECT empresa_id FROM usuarios_empresas ue INNER JOIN empresas e ON e.id = ue.empresa_id WHERE ue.usuario_id = ?1 AND e.ativo = 1 ORDER BY e.id LIMIT 1",
        [identity.user_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| format!("Falha ao resolver empresa ativa do usuário: {err}"))?
    .map(Some)
    .ok_or_else(|| "Usuário não possui empresa ativa vinculada.".to_string())
}

fn required_company(
    conn: &Connection,
    identity: &SessionIdentity,
    requested: Option<i64>,
) -> Result<i64, String> {
    resolve_company(conn, identity, requested)?
        .ok_or_else(|| "Selecione uma empresa para executar esta operação.".to_string())
}

fn filters_company(filters: &Map<String, Value>) -> Option<i64> {
    filters
        .get("empresa_id")
        .or_else(|| filters.get("empresaId"))
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
}

fn value_text(value: Option<&Value>) -> String {
    value
        .and_then(|value| match value {
            Value::String(text) => Some(text.trim().to_string()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn manifestador_summary(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: Option<i64>,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = resolve_company(&conn, &identity, empresa_id)?;
    manifestador::summary(&conn, company_id)
}

#[tauri::command]
pub fn manifestador_config_get(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
) -> Result<Value, String> {
    if empresa_id <= 0 {
        return Err("Informe a empresa dos parâmetros fiscais.".into());
    }
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    manifestador::get_config(&conn, company_id)
}

#[tauri::command]
pub fn manifestador_config_save(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    payload: Map<String, Value>,
) -> Result<Value, String> {
    if empresa_id <= 0 {
        return Err("Informe a empresa dos parâmetros fiscais.".into());
    }
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    manifestador::save_config(&conn, company_id, &payload)
}

#[tauri::command]
pub fn manifestador_company_config_get(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    manifestador::get_company_config(&conn, company_id)
}

#[tauri::command]
pub fn manifestador_company_config_save(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    payload: Map<String, Value>,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    manifestador::save_company_config(&conn, company_id, &payload)
}

#[tauri::command]
pub fn manifestador_notes_list(
    state: State<'_, SharedState>,
    session_token: String,
    mut filters: Map<String, Value>,
) -> Result<Vec<Map<String, Value>>, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = resolve_company(&conn, &identity, filters_company(&filters))?;
    if let Some(company_id) = company_id {
        filters.insert("empresa_id".to_string(), Value::from(company_id));
    }
    manifestador::list_notes(&conn, &filters)
}

#[tauri::command]
pub fn manifestador_note_get(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    nota_id: i64,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let valid: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
            params![nota_id, company_id],
            |row| row.get(0),
        )
        .map_err(|err| format!("Falha ao validar NF-e: {err}"))?;
    if valid == 0 {
        return Err("NF-e não pertence à empresa ativa.".to_string());
    }
    manifestador::note_json(&conn, nota_id)
}

#[tauri::command]
pub fn manifestador_note_logs(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    nota_id: i64,
) -> Result<Vec<Map<String, Value>>, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let valid: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
            params![nota_id, company_id],
            |row| row.get(0),
        )
        .map_err(|err| format!("Falha ao validar histórico: {err}"))?;
    if valid == 0 {
        return Err("NF-e não pertence à empresa ativa.".to_string());
    }
    manifestador::list_logs(&conn, nota_id)
}

#[tauri::command]
pub fn manifestador_import_xml(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    xml: String,
    nsu: Option<i64>,
) -> Result<Value, String> {
    let (mut conn, identity) =
        session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    manifestador::import_xml(&mut conn, company_id, &xml, nsu.unwrap_or_default(), None)
}

#[tauri::command]
pub fn manifestador_obter_notas(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    data_inicial: Option<String>,
    data_final: Option<String>,
) -> Result<Value, String> {
    let (mut conn, identity) =
        session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let request = json!({
        "empresa_id": company_id,
        "data_inicial": data_inicial,
        "data_final": data_final,
    });
    let (response, mode) =
        if let Some(response) = manifestador::call_provider(&conn, "obter_notas", &request)? {
            (response, "provider")
        } else {
            (
                manifestador::native_obter_notas(&conn, company_id)?,
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
        if manifestador::import_provider_document(&mut conn, company_id, &document)? {
            imported += 1;
        }
    }
    Ok(
        json!({"ok": true, "mode": mode, "imported": imported, "message": format!("{imported} documento(s) recebido(s) pela integração {mode}. ")}),
    )
}

#[tauri::command]
pub fn manifestador_download(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    nota_id: i64,
    xml: Option<String>,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let data_dir = state.data_dir()?;
    if let Some(xml) = xml.as_deref().filter(|value| !value.trim().is_empty()) {
        return manifestador::download_note(&conn, &data_dir, company_id, nota_id, Some(xml));
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
    if let Some(response) = manifestador::call_provider(&conn, "download", &request)? {
        let mut full_xml = value_text(response.get("xml"));
        if full_xml.is_empty() {
            full_xml = value_text(response.get("docZip"));
            if !full_xml.is_empty() {
                full_xml = manifestador::decode_doczip(&full_xml)?;
            }
        }
        if full_xml.trim().is_empty() {
            return Err("O adaptador externo não retornou o XML completo da NF-e.".to_string());
        }
        return manifestador::download_note(&conn, &data_dir, company_id, nota_id, Some(&full_xml));
    }
    manifestador::native_download_note(&conn, company_id, nota_id, &data_dir)
}

#[tauri::command]
pub fn manifestador_manifestar(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    nota_id: i64,
    evento: String,
    justificativa: Option<String>,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let chave: String = conn
        .query_row(
            "SELECT chave FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
            params![nota_id, company_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao localizar NF-e para manifestação: {err}"))?
        .ok_or_else(|| "NF-e não pertence à empresa ativa.".to_string())?;
    let request = json!({"empresa_id": company_id, "nota_id": nota_id, "chave": chave, "evento": evento, "justificativa": justificativa});
    if let Some(response) = manifestador::call_provider(&conn, "manifestar", &request)? {
        let protocol = response.get("protocolo").and_then(Value::as_str);
        let result = manifestador::update_manifestation(
            &conn,
            nota_id,
            &evento,
            justificativa.as_deref().unwrap_or_default(),
            protocol,
        )?;
        return Ok(
            json!({"mode": "provider", "response": response, "note": result, "message": "Manifestação enviada ao adaptador e registrada localmente."}),
        );
    }
    let result = manifestador::update_manifestation(
        &conn,
        nota_id,
        &evento,
        justificativa.as_deref().unwrap_or_default(),
        None,
    )?;
    Ok(
        json!({"mode": "local", "note": result, "warning": "Manifestação registrada localmente. Configure um adaptador externo para assinatura e envio real à SEFAZ."}),
    )
}

#[tauri::command]
pub fn manifestador_export_zip(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: Option<i64>,
    filters: Option<Map<String, Value>>,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = resolve_company(&conn, &identity, empresa_id)?;
    let data_dir = state.data_dir()?;
    manifestador::export_zip(&conn, &data_dir, company_id, &filters.unwrap_or_default())
}

#[tauri::command]
pub fn manifestador_send_accounting(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    data_inicial: String,
    data_final: String,
    destinatario: String,
) -> Result<Value, String> {
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:manage")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    if data_inicial.trim().is_empty() || data_final.trim().is_empty() {
        return Err("Informe o período para envio à contabilidade.".to_string());
    }
    if destinatario.trim().is_empty() {
        return Err("Informe o e-mail destinatário da contabilidade.".to_string());
    }
    let data_dir = state.data_dir()?;
    manifestador::send_accounting_email(
        &conn,
        &data_dir,
        company_id,
        data_inicial.trim(),
        data_final.trim(),
        destinatario.trim(),
    )
}

#[tauri::command]
pub fn manifestador_provider_test(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
) -> Result<Value, String> {
    if empresa_id <= 0 {
        return Err("Informe a empresa dos parâmetros fiscais.".into());
    }
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company_id = required_company(&conn, &identity, Some(empresa_id))?;
    let Some((endpoint, timeout)) = manifestador::provider_endpoint(&conn, company_id)? else {
        return Ok(json!({"configured": false, "message": "Nenhum endpoint externo configurado."}));
    };
    Ok(
        json!({"configured": true, "endpoint": endpoint, "timeout_seconds": timeout, "message": "Endpoint configurado. O teste efetivo ocorre ao solicitar notas ou enviar manifestação."}),
    )
}

#[tauri::command]
pub fn application_smtp_get(
    state: State<'_, SharedState>,
    session_token: String,
) -> Result<Value, String> {
    let (conn, _) = session_and_connection(&state, &session_token, "config:view")?;
    crate::smtp_settings::get(&conn)
}

#[tauri::command]
pub fn application_smtp_save(
    state: State<'_, SharedState>,
    session_token: String,
    payload: Map<String, Value>,
) -> Result<Value, String> {
    let (conn, _) = session_and_connection(&state, &session_token, "config:manage")?;
    crate::smtp_settings::save(&conn, &payload)
}

#[tauri::command]
pub fn manifestador_fiscal_status(
    state: State<'_, SharedState>,
    session_token: String,
) -> Result<Value, String> {
    session_and_connection(&state, &session_token, "manifestador:view")?;
    Ok(crate::fiscal_engine::status())
}

#[tauri::command]
pub fn manifestador_fiscal_process(
    state: State<'_, SharedState>,
    session_token: String,
    empresa_id: i64,
    nota_id: i64,
    operation: String,
) -> Result<Value, String> {
    if empresa_id <= 0 {
        return Err("Selecione uma empresa.".into());
    }
    let (conn, identity) = session_and_connection(&state, &session_token, "manifestador:view")?;
    let company = required_company(&conn, &identity, Some(empresa_id))?;
    crate::fiscal_engine::process(&conn, &state.data_dir()?, company, nota_id, &operation)
}
