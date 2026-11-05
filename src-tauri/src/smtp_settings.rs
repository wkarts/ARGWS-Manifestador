//! SMTP global independente dos parâmetros fiscais e das credenciais de empresa.
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use crate::security::{encrypt_text, machine_key};

pub fn migrate(conn: &Connection) -> Result<(), String> {
    // Não reativa credenciais antigas de manifestador_config: aquele registro é histórico.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS application_smtp (
        id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL DEFAULT 0,
        smtp_host TEXT NOT NULL DEFAULT '', smtp_conta TEXT NOT NULL DEFAULT '',
        smtp_remetente TEXT NOT NULL DEFAULT '', smtp_email TEXT NOT NULL DEFAULT '',
        smtp_senha_encrypted TEXT, smtp_porta INTEGER NOT NULL DEFAULT 587,
        updated_at TEXT);
        INSERT OR IGNORE INTO application_smtp(id) VALUES(1);",
    )
    .map_err(|e| format!("Falha ao preparar SMTP global: {e}"))
}

pub fn get(conn: &Connection) -> Result<Value, String> {
    conn.query_row(
        "SELECT enabled, smtp_host, smtp_conta, smtp_remetente, smtp_email,
        smtp_porta, COALESCE(smtp_senha_encrypted,'') <> '' FROM application_smtp WHERE id=1",
        [],
        |r| {
            Ok(
                json!({"enabled":r.get::<_,bool>(0)?,"smtp_host":r.get::<_,String>(1)?,
            "smtp_conta":r.get::<_,String>(2)?,"smtp_remetente":r.get::<_,String>(3)?,
            "smtp_email":r.get::<_,String>(4)?,"smtp_porta":r.get::<_,i64>(5)?,
            "smtp_password_configured":r.get::<_,bool>(6)?}),
            )
        },
    )
    .map_err(|e| format!("Falha ao carregar SMTP global: {e}"))
}

pub fn save(conn: &Connection, payload: &Map<String, Value>) -> Result<Value, String> {
    let current = get(conn)?;
    let value = |key: &str| {
        payload
            .get(key)
            .or_else(|| current.get(key))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let enabled = payload
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(current["enabled"].as_bool().unwrap_or(false));
    let host = value("smtp_host");
    let email = value("smtp_email");
    let account = value("smtp_conta");
    let sender = value("smtp_remetente");
    let port = payload
        .get("smtp_porta")
        .or_else(|| current.get("smtp_porta"))
        .and_then(Value::as_i64)
        .ok_or("Porta SMTP inválida.")?;
    if !(1..=65535).contains(&port) {
        return Err("Porta SMTP deve estar entre 1 e 65535.".into());
    }
    if [&host, &email, &account, &sender]
        .iter()
        .any(|s| s.contains(['\r', '\n', '\0']))
    {
        return Err("Campos SMTP não podem conter quebras de linha.".into());
    }
    if enabled && (host.is_empty() || !email.contains('@') || email.contains(' ')) {
        return Err("Informe servidor e e-mail do remetente para ativar o SMTP global.".into());
    }
    // Não retorna a senha, nem o texto criptografado. Campo vazio preserva a senha salva.
    let password = payload
        .get("smtp_password")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(|v| encrypt_text(&machine_key(), v));
    conn.execute(
        "UPDATE application_smtp SET enabled=?1, smtp_host=?2, smtp_conta=?3,
        smtp_remetente=?4, smtp_email=?5, smtp_porta=?6,
        smtp_senha_encrypted=COALESCE(?7,smtp_senha_encrypted), updated_at=?8 WHERE id=1",
        params![
            enabled,
            host,
            account,
            sender,
            email,
            port,
            password,
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| format!("Falha ao salvar SMTP global: {e}"))?;
    get(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_migration_and_secret_preserving_updates() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        assert_eq!(get(&conn).unwrap()["enabled"], false);
        save(&conn,json!({"enabled":true,"smtp_host":"mail.example","smtp_email":"sender@example.com","smtp_password":" secret "}).as_object().unwrap()).unwrap();
        migrate(&conn).unwrap();
        let result = save(&conn, json!({"smtp_password":""}).as_object().unwrap()).unwrap();
        assert_eq!(result["smtp_password_configured"], true);
        assert!(result.get("smtp_senha_encrypted").is_none());
        let cipher: String = conn
            .query_row(
                "SELECT smtp_senha_encrypted FROM application_smtp",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            crate::security::decrypt_text(&machine_key(), &cipher).unwrap(),
            " secret "
        );
        assert!(save(&conn, json!({"smtp_porta":65536}).as_object().unwrap()).is_err());
        assert!(save(
            &conn,
            json!({"smtp_email":"a@b\r\nBcc: x@y"}).as_object().unwrap()
        )
        .is_err());
        assert_eq!(get(&conn).unwrap()["smtp_porta"], 587);
    }
}
