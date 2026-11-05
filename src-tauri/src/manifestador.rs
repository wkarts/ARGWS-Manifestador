//! Núcleo funcional do Manifestador NF-e.
//!
//! A aplicação nova usa SQLite local, preserva a migração dos dados legados e
//! oferece duas integrações explícitas: adaptador JSON compatível com ACBr/SEFAZ
//! e consulta nativa DistribuicaoDFe com certificado A1/PFX no desktop. Eventos
//! de manifestação só são considerados enviados quando um provedor retorna a
//! resposta fiscal; sem isso, o histórico local é preservado sem simulação.

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::Utc;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use url::Url;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    db::row_to_json_map,
    security::{decrypt_text, encrypt_text, machine_key},
};

const CONFIG_ID: i64 = 1;
const DEFAULT_TIMEOUT_SECONDS: i64 = 60;

type SmtpConfigRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

#[cfg(feature = "desktop")]
type NativeCompanyConfig = (String, String, String, String, String, String, String, u64);

#[cfg(feature = "desktop")]
type NativeCompanyConfigRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedNfe {
    pub chave: String,
    pub numero: i64,
    pub serie: i64,
    pub cnpj: String,
    pub nome: String,
    pub ie: String,
    pub emissao: String,
    pub valor: f64,
    pub tipo: String,
    pub ambiente: String,
    pub total_produtos: f64,
    pub valor_frete: f64,
    pub valor_despesas: f64,
    pub valor_desconto: f64,
    pub valor_ipi: f64,
    pub valor_icms: f64,
    pub valor_icms_st: f64,
    pub endereco: String,
    pub endereco_numero: String,
    pub endereco_complemento: String,
    pub endereco_bairro: String,
    pub endereco_cep: String,
    pub endereco_cidade: String,
    pub endereco_codmunicipio: String,
    pub telefone: String,
    pub protocolo: String,
    pub items: Vec<ParsedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedItem {
    pub item: i64,
    pub cprod: String,
    pub xprod: String,
    pub ncm: String,
    pub cfop: String,
    pub ucom: String,
    pub qcom: f64,
    pub utrib: String,
    pub qtrib: f64,
    pub vuntrib: f64,
    pub orig: String,
    pub csosn: String,
    pub st: String,
    pub vbcicms: f64,
    pub picms: f64,
    pub vicms: f64,
    pub vicmsst: f64,
    pub vbcicmsst: f64,
    pub vipi: f64,
    pub pipi: f64,
}

#[derive(Debug, Clone)]
struct LegacyEmpresa {
    codigo: i64,
    cnpj: String,
    certificado: String,
    senha: String,
    uf: String,
    caminho: String,
    nome: String,
    numero_serie: String,
    caminho_copia_xml: String,
    data_manifestacao: String,
}

#[derive(Debug, Clone)]
struct LegacyNota {
    codigo: i64,
    id_empresa: i64,
    numero: i64,
    serie: i64,
    cnpj: String,
    nome: String,
    emissao: String,
    valor: f64,
    tipo: String,
    situacao: String,
    chave: String,
    ie: String,
    protocolo: String,
    datarec: String,
    manifestacao: String,
    download: String,
    ambiente: String,
    nsu: i64,
    visivel: String,
    total_produtos: f64,
    valor_frete: f64,
    valor_despesas: f64,
    valor_desconto: f64,
    valor_ipi: f64,
    valor_icms: f64,
    valor_icmsst: f64,
    endereco: String,
    endereco_numero: String,
    endereco_complemento: String,
    endereco_bairro: String,
    endereco_cep: String,
    endereco_cidade: String,
    endereco_codmunicipio: String,
    telefone: String,
}

pub fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS manifestador_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            verificar_automatico INTEGER NOT NULL DEFAULT 1,
            download_automatico INTEGER NOT NULL DEFAULT 1,
            timer_minutos INTEGER NOT NULL DEFAULT 60,
            smtp_host TEXT,
            smtp_conta TEXT,
            smtp_remetente TEXT,
            smtp_email TEXT,
            smtp_senha_encrypted TEXT,
            smtp_porta INTEGER NOT NULL DEFAULT 587,
            sefaz_endpoint TEXT,
            sefaz_service_url TEXT,
            sefaz_timeout_seconds INTEGER NOT NULL DEFAULT 60,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS manifestador_empresa_config (
            empresa_id INTEGER PRIMARY KEY,
            certificado_path TEXT,
            certificado_password_encrypted TEXT,
            uf TEXT,
            xml_path TEXT,
            numero_serie TEXT,
            copia_xml_path TEXT,
            resposta_path TEXT,
            ambiente TEXT NOT NULL DEFAULT 'production',
            nsu TEXT NOT NULL DEFAULT '0',
            data_manifestacao TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (empresa_id) REFERENCES empresas(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS manifestador_notas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            empresa_id INTEGER NOT NULL,
            numero INTEGER NOT NULL DEFAULT 0,
            serie INTEGER NOT NULL DEFAULT 0,
            cnpj TEXT NOT NULL DEFAULT '',
            nome TEXT NOT NULL DEFAULT '',
            emissao TEXT,
            valor REAL NOT NULL DEFAULT 0,
            tipo TEXT NOT NULL DEFAULT 'NFe',
            situacao TEXT NOT NULL DEFAULT 'recebida',
            chave TEXT NOT NULL,
            ie TEXT,
            protocolo TEXT,
            data_recebimento TEXT,
            manifestacao TEXT NOT NULL DEFAULT 'N',
            download INTEGER NOT NULL DEFAULT 0,
            download_path TEXT,
            ambiente TEXT NOT NULL DEFAULT 'production',
            nsu INTEGER NOT NULL DEFAULT 0,
            visivel INTEGER NOT NULL DEFAULT 1,
            total_produtos REAL NOT NULL DEFAULT 0,
            valor_frete REAL NOT NULL DEFAULT 0,
            valor_despesas REAL NOT NULL DEFAULT 0,
            valor_desconto REAL NOT NULL DEFAULT 0,
            valor_ipi REAL NOT NULL DEFAULT 0,
            valor_icms REAL NOT NULL DEFAULT 0,
            valor_icms_st REAL NOT NULL DEFAULT 0,
            endereco TEXT,
            endereco_numero TEXT,
            endereco_complemento TEXT,
            endereco_bairro TEXT,
            endereco_cep TEXT,
            endereco_cidade TEXT,
            endereco_codmunicipio TEXT,
            telefone TEXT,
            xml_content TEXT,
            xml_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (empresa_id) REFERENCES empresas(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS manifestador_itens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nota_id INTEGER NOT NULL,
            item INTEGER NOT NULL DEFAULT 0,
            cprod TEXT,
            xprod TEXT,
            ncm TEXT,
            cfop TEXT,
            ucom TEXT,
            qcom REAL NOT NULL DEFAULT 0,
            utrib TEXT,
            qtrib REAL NOT NULL DEFAULT 0,
            vuntrib REAL NOT NULL DEFAULT 0,
            orig TEXT,
            csosn TEXT,
            st TEXT,
            vbcicms REAL NOT NULL DEFAULT 0,
            picms REAL NOT NULL DEFAULT 0,
            vicms REAL NOT NULL DEFAULT 0,
            vicmsst REAL NOT NULL DEFAULT 0,
            vbcicmsst REAL NOT NULL DEFAULT 0,
            vipi REAL NOT NULL DEFAULT 0,
            pipi REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (nota_id) REFERENCES manifestador_notas(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS manifestador_nf_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nota_id INTEGER NOT NULL,
            operacao INTEGER NOT NULL DEFAULT 0,
            datahora TEXT NOT NULL,
            historico TEXT NOT NULL,
            FOREIGN KEY (nota_id) REFERENCES manifestador_notas(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS manifestador_legacy_empresa_map (
            legacy_codigo INTEGER PRIMARY KEY,
            empresa_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (empresa_id) REFERENCES empresas(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS manifestador_legacy_nf_map (
            legacy_codigo INTEGER PRIMARY KEY,
            nota_id INTEGER NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            FOREIGN KEY (nota_id) REFERENCES manifestador_notas(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS ux_manifestador_nota_empresa_chave
            ON manifestador_notas(empresa_id, chave, ambiente);
        CREATE UNIQUE INDEX IF NOT EXISTS ux_manifestador_item_nota_item
            ON manifestador_itens(nota_id, item);
        CREATE INDEX IF NOT EXISTS idx_manifestador_nota_empresa
            ON manifestador_notas(empresa_id, emissao DESC);
        CREATE INDEX IF NOT EXISTS idx_manifestador_nota_situacao
            ON manifestador_notas(empresa_id, situacao, manifestacao);
        CREATE INDEX IF NOT EXISTS idx_manifestador_nota_nsu
            ON manifestador_notas(empresa_id, ambiente, nsu);
        CREATE INDEX IF NOT EXISTS idx_manifestador_log_nota
            ON manifestador_nf_log(nota_id, datahora DESC);
        "#,
    )
    .map_err(|err| format!("Falha ao criar tabelas do Manifestador: {err}"))?;

    ensure_column(conn, "manifestador_config", "sefaz_service_url", "TEXT")?;

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO manifestador_config (id, created_at, updated_at) VALUES (?1, ?2, ?2)",
        params![CONFIG_ID, now],
    )
    .map_err(|err| format!("Falha ao criar configuração padrão do Manifestador: {err}"))?;

    migrate_legacy_companies(conn)?;
    migrate_legacy_config(conn)?;
    migrate_legacy_notes(conn)?;
    migrate_company_operation_config(conn)?;
    Ok(())
}

fn migrate_company_operation_config(conn: &Connection) -> Result<(), String> {
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='manifestador_empresa_operacao')", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if exists {
        return Ok(());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(
        r#"        CREATE TABLE IF NOT EXISTS manifestador_empresa_operacao (
            empresa_id INTEGER PRIMARY KEY REFERENCES empresas(id) ON DELETE CASCADE,
            last_check_at INTEGER,
            verificar_automatico INTEGER NOT NULL DEFAULT 1,
            download_automatico INTEGER NOT NULL DEFAULT 1,
            timer_minutos INTEGER NOT NULL DEFAULT 60,
            smtp_host TEXT,
            smtp_conta TEXT,
            smtp_remetente TEXT,
            smtp_email TEXT,
            smtp_senha_encrypted TEXT,
            smtp_porta INTEGER NOT NULL DEFAULT 587,
            sefaz_endpoint TEXT,
            sefaz_service_url TEXT,
            sefaz_timeout_seconds INTEGER NOT NULL DEFAULT 60,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );"#,
    )
    .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO manifestador_empresa_operacao (empresa_id, verificar_automatico, download_automatico, timer_minutos, smtp_host, smtp_conta, smtp_remetente, smtp_email, smtp_senha_encrypted, smtp_porta, sefaz_endpoint, sefaz_service_url, sefaz_timeout_seconds, created_at, updated_at) SELECT e.id, g.verificar_automatico, g.download_automatico, g.timer_minutos, g.smtp_host, g.smtp_conta, g.smtp_remetente, g.smtp_email, g.smtp_senha_encrypted, g.smtp_porta, g.sefaz_endpoint, g.sefaz_service_url, g.sefaz_timeout_seconds, g.created_at, g.updated_at FROM empresas e CROSS JOIN manifestador_config g WHERE g.id = 1", []).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

fn ensure_operation_config(conn: &Connection, empresa_id: i64) -> Result<(), String> {
    conn.execute("INSERT OR IGNORE INTO manifestador_empresa_operacao (empresa_id, created_at, updated_at) SELECT id, ?2, ?2 FROM empresas WHERE id = ?1", params![empresa_id, Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ensure_company_configs(conn: &Connection) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare("SELECT id FROM empresas WHERE ativo = 1 ORDER BY id")
        .map_err(|err| format!("Falha ao preparar configuração fiscal das empresas: {err}"))?;
    let ids = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|err| format!("Falha ao consultar empresas do Manifestador: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear empresas do Manifestador: {err}"))?;
    for id in ids {
        ensure_operation_config(conn, id)?;
        conn.execute(
            "INSERT OR IGNORE INTO manifestador_empresa_config (empresa_id, created_at, updated_at) VALUES (?1, ?2, ?2)",
            params![id, now],
        )
        .map_err(|err| format!("Falha ao criar configuração da empresa {id}: {err}"))?;
    }
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND lower(name) = lower(?1)",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .map_err(|err| format!("Falha ao verificar tabela legada {table}: {err}"))
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|err| format!("Falha ao inspecionar coluna {table}.{column}: {err}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("Falha ao consultar coluna {table}.{column}: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear coluna {table}.{column}: {err}"))?;
    if columns
        .iter()
        .any(|value| value.eq_ignore_ascii_case(column))
    {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )
    .map_err(|err| format!("Falha ao adicionar coluna {table}.{column}: {err}"))?;
    Ok(())
}

fn row_text(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Null => Ok(String::new()),
        rusqlite::types::ValueRef::Text(value) => {
            Ok(String::from_utf8_lossy(value).trim().to_string())
        }
        rusqlite::types::ValueRef::Integer(value) => Ok(value.to_string()),
        rusqlite::types::ValueRef::Real(value) => Ok(value.to_string()),
        rusqlite::types::ValueRef::Blob(value) => {
            Ok(String::from_utf8_lossy(value).trim().to_string())
        }
    }
}

fn row_i64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<i64> {
    Ok(match row.get_ref(index)? {
        rusqlite::types::ValueRef::Integer(value) => value,
        rusqlite::types::ValueRef::Real(value) => value as i64,
        rusqlite::types::ValueRef::Text(value) => String::from_utf8_lossy(value)
            .trim()
            .parse()
            .unwrap_or_default(),
        _ => 0,
    })
}

fn row_f64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<f64> {
    Ok(match row.get_ref(index)? {
        rusqlite::types::ValueRef::Integer(value) => value as f64,
        rusqlite::types::ValueRef::Real(value) => value,
        rusqlite::types::ValueRef::Text(value) => String::from_utf8_lossy(value)
            .trim()
            .replace(',', ".")
            .parse()
            .unwrap_or_default(),
        _ => 0.0,
    })
}

fn normalize_document(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect()
}

fn uf_from_legacy_code(value: &str) -> String {
    let code = value.trim().parse::<i64>().unwrap_or_default();
    match code {
        11 => "RO",
        12 => "AC",
        13 => "AM",
        14 => "RR",
        15 => "PA",
        16 => "AP",
        17 => "TO",
        21 => "MA",
        22 => "PI",
        23 => "CE",
        24 => "RN",
        25 => "PB",
        26 => "PE",
        27 => "AL",
        28 => "SE",
        29 => "BA",
        31 => "MG",
        32 => "ES",
        33 => "RJ",
        35 => "SP",
        41 => "PR",
        42 => "SC",
        43 => "RS",
        50 => "MS",
        51 => "MT",
        52 => "GO",
        53 => "DF",
        _ => value.trim(),
    }
    .to_string()
}

fn migrate_legacy_companies(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "EMPRESA")? {
        return Ok(());
    }
    let mut stmt = conn
        .prepare("SELECT CODIGO, CNPJ, CERTIFICADO, SENHA, UF, CAMINHO, NOME, NUMERO_SERIE, CAMINHO_COPIA_XML, DATA_MANIFESTACAO FROM EMPRESA ORDER BY CODIGO")
        .map_err(|err| format!("Falha ao preparar empresas Delphi: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyEmpresa {
                codigo: row_i64(row, 0)?,
                cnpj: row_text(row, 1)?,
                certificado: row_text(row, 2)?,
                senha: row_text(row, 3)?,
                uf: uf_from_legacy_code(&row_text(row, 4)?),
                caminho: row_text(row, 5)?,
                nome: row_text(row, 6)?,
                numero_serie: row_text(row, 7)?,
                caminho_copia_xml: row_text(row, 8)?,
                data_manifestacao: row_text(row, 9)?,
            })
        })
        .map_err(|err| format!("Falha ao ler empresas Delphi: {err}"))?;

    let key = machine_key();
    for row in rows {
        let legacy = row.map_err(|err| format!("Falha ao mapear empresa Delphi: {err}"))?;
        let existing: Option<i64> = conn
            .query_row(
                "SELECT empresa_id FROM manifestador_legacy_empresa_map WHERE legacy_codigo = ?1",
                [legacy.codigo],
                |item| item.get(0),
            )
            .optional()
            .map_err(|err| format!("Falha ao consultar mapa de empresa Delphi: {err}"))?;
        let empresa_id = if let Some(id) = existing {
            id
        } else {
            let documento = normalize_document(&legacy.cnpj);
            let by_document: Option<i64> = conn
                .query_row(
                    "SELECT id FROM empresas WHERE documento = ?1 LIMIT 1",
                    [documento.as_str()],
                    |item| item.get(0),
                )
                .optional()
                .map_err(|err| format!("Falha ao localizar empresa migrada: {err}"))?;
            let id = if let Some(id) = by_document {
                id
            } else {
                conn.execute(
                    "INSERT INTO empresas (nome, nome_fantasia, documento, estado, ativo, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                    params![
                        if legacy.nome.is_empty() { legacy.cnpj.clone() } else { legacy.nome.clone() },
                        legacy.nome.clone(),
                        documento,
                        legacy.uf.clone(),
                        Utc::now().to_rfc3339(),
                    ],
                )
                .map_err(|err| format!("Falha ao migrar empresa legada: {err}"))?;
                conn.last_insert_rowid()
            };
            conn.execute(
                "INSERT OR REPLACE INTO manifestador_legacy_empresa_map (legacy_codigo, empresa_id, created_at) VALUES (?1, ?2, ?3)",
                params![legacy.codigo, id, Utc::now().to_rfc3339()],
            )
            .map_err(|err| format!("Falha ao gravar mapa da empresa legada: {err}"))?;
            id
        };

        conn.execute(
            "INSERT OR IGNORE INTO manifestador_empresa_config (empresa_id, certificado_path, certificado_password_encrypted, uf, xml_path, numero_serie, copia_xml_path, data_manifestacao, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                empresa_id,
                empty_to_none(&legacy.certificado),
                encrypted_optional(&key, &legacy.senha),
                empty_to_none(&legacy.uf),
                empty_to_none(&legacy.caminho),
                empty_to_none(&legacy.numero_serie),
                empty_to_none(&legacy.caminho_copia_xml),
                empty_to_none(&legacy.data_manifestacao),
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|err| format!("Falha ao migrar configuração da empresa Delphi: {err}"))?;
    }
    Ok(())
}

fn migrate_legacy_config(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "CFG")? {
        return Ok(());
    }
    let row = conn
        .query_row(
            "SELECT VERIFICAR_AUTOMATICO, DOWNLOAD_AUTOMATICO, TIMER, SMTP, SMTP_CONTA, SMTP_REMETENTE, SMTP_EMAIL, SMTP_SENHA, SMTP_PORTA FROM CFG ORDER BY CODIGO LIMIT 1",
            [],
            |item| {
                Ok((
                    row_text(item, 0)?, row_text(item, 1)?, row_i64(item, 2)?, row_text(item, 3)?,
                    row_text(item, 4)?, row_text(item, 5)?, row_text(item, 6)?, row_text(item, 7)?, row_i64(item, 8)?,
                ))
            },
        )
        .optional()
        .map_err(|err| format!("Falha ao ler configuração Delphi: {err}"))?;
    let Some((verify, download, timer, smtp, account, sender, email, password, port)) = row else {
        return Ok(());
    };
    let key = machine_key();
    conn.execute(
        "UPDATE manifestador_config SET verificar_automatico = ?1, download_automatico = ?2, timer_minutos = ?3, smtp_host = ?4, smtp_conta = ?5, smtp_remetente = ?6, smtp_email = ?7, smtp_senha_encrypted = ?8, smtp_porta = ?9, updated_at = ?10 WHERE id = 1",
        params![
            bool_from_legacy(&verify), bool_from_legacy(&download), if timer > 0 { timer } else { 60 },
            empty_to_none(&smtp), empty_to_none(&account), empty_to_none(&sender), empty_to_none(&email),
            encrypted_optional(&key, &password), if port > 0 { port } else { 587 }, Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|err| format!("Falha ao migrar configuração geral Delphi: {err}"))?;
    Ok(())
}

fn migrate_legacy_notes(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "NF")? {
        return Ok(());
    }
    let mut stmt = conn
        .prepare("SELECT CODIGO, ID_EMPRESA, NUMERO, SERIE, CNPJ, NOME, EMISSAO, VALOR, TIPO, SITUACAO, CHAVE, IE, PROTOCOLO, DATAREC, MANIFESTACAO, DOWNLOAD, AMBIENTE, NSU, VISIVEL, TOTAL_PRODUTOS, VALOR_FRETE, VALOR_DESPESAS, VALOR_DESCONTO, VALOR_IPI, VALOR_ICMS, VALOR_ICMSST, ENDERECO, ENDERECO_NUMERO, ENDERECO_COMPLEMENTO, ENDERECO_BAIRRO, ENDERECO_CEP, ENDERECO_CIDADE, ENDERECO_CODMUNICIPIO, TELEFONE FROM NF ORDER BY CODIGO")
        .map_err(|err| format!("Falha ao preparar notas Delphi: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyNota {
                codigo: row_i64(row, 0)?,
                id_empresa: row_i64(row, 1)?,
                numero: row_i64(row, 2)?,
                serie: row_i64(row, 3)?,
                cnpj: row_text(row, 4)?,
                nome: row_text(row, 5)?,
                emissao: row_text(row, 6)?,
                valor: row_f64(row, 7)?,
                tipo: row_text(row, 8)?,
                situacao: row_text(row, 9)?,
                chave: row_text(row, 10)?,
                ie: row_text(row, 11)?,
                protocolo: row_text(row, 12)?,
                datarec: row_text(row, 13)?,
                manifestacao: row_text(row, 14)?,
                download: row_text(row, 15)?,
                ambiente: environment_from_legacy(&row_text(row, 16)?),
                nsu: row_i64(row, 17)?,
                visivel: row_text(row, 18)?,
                total_produtos: row_f64(row, 19)?,
                valor_frete: row_f64(row, 20)?,
                valor_despesas: row_f64(row, 21)?,
                valor_desconto: row_f64(row, 22)?,
                valor_ipi: row_f64(row, 23)?,
                valor_icms: row_f64(row, 24)?,
                valor_icmsst: row_f64(row, 25)?,
                endereco: row_text(row, 26)?,
                endereco_numero: row_text(row, 27)?,
                endereco_complemento: row_text(row, 28)?,
                endereco_bairro: row_text(row, 29)?,
                endereco_cep: row_text(row, 30)?,
                endereco_cidade: row_text(row, 31)?,
                endereco_codmunicipio: row_text(row, 32)?,
                telefone: row_text(row, 33)?,
            })
        })
        .map_err(|err| format!("Falha ao ler notas Delphi: {err}"))?;
    for row in rows {
        let legacy = row.map_err(|err| format!("Falha ao mapear nota Delphi: {err}"))?;
        let empresa_id: Option<i64> = conn
            .query_row(
                "SELECT empresa_id FROM manifestador_legacy_empresa_map WHERE legacy_codigo = ?1",
                [legacy.id_empresa],
                |item| item.get(0),
            )
            .optional()
            .map_err(|err| format!("Falha ao localizar empresa da nota Delphi: {err}"))?;
        let Some(empresa_id) = empresa_id else {
            continue;
        };
        let existing: Option<i64> = conn
            .query_row(
                "SELECT nota_id FROM manifestador_legacy_nf_map WHERE legacy_codigo = ?1",
                [legacy.codigo],
                |item| item.get(0),
            )
            .optional()
            .map_err(|err| format!("Falha ao consultar mapa de nota Delphi: {err}"))?;
        let note_id = if let Some(id) = existing {
            id
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO manifestador_notas (empresa_id, numero, serie, cnpj, nome, emissao, valor, tipo, situacao, chave, ie, protocolo, data_recebimento, manifestacao, download, ambiente, nsu, visivel, total_produtos, valor_frete, valor_despesas, valor_desconto, valor_ipi, valor_icms, valor_icms_st, endereco, endereco_numero, endereco_complemento, endereco_bairro, endereco_cep, endereco_cidade, endereco_codmunicipio, telefone, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35)",
                params![empresa_id, legacy.numero, legacy.serie, normalize_document(&legacy.cnpj), legacy.nome.clone(), empty_to_none(&legacy.emissao), legacy.valor, default_text(&legacy.tipo, "NFe"), default_text(&legacy.situacao, "recebida"), legacy.chave.clone(), empty_to_none(&legacy.ie), empty_to_none(&legacy.protocolo), empty_to_none(&legacy.datarec), default_text(&legacy.manifestacao, "N"), bool_from_legacy(&legacy.download), legacy.ambiente.clone(), legacy.nsu, bool_from_legacy(&legacy.visivel), legacy.total_produtos, legacy.valor_frete, legacy.valor_despesas, legacy.valor_desconto, legacy.valor_ipi, legacy.valor_icms, legacy.valor_icmsst, empty_to_none(&legacy.endereco), empty_to_none(&legacy.endereco_numero), empty_to_none(&legacy.endereco_complemento), empty_to_none(&legacy.endereco_bairro), empty_to_none(&legacy.endereco_cep), empty_to_none(&legacy.endereco_cidade), empty_to_none(&legacy.endereco_codmunicipio), empty_to_none(&legacy.telefone), Utc::now().to_rfc3339(), Utc::now().to_rfc3339()],
            ).map_err(|err| format!("Falha ao migrar nota Delphi: {err}"))?;
            let id: i64 = conn.query_row("SELECT id FROM manifestador_notas WHERE empresa_id = ?1 AND chave = ?2 AND ambiente = ?3", params![empresa_id, legacy.chave, legacy.ambiente], |item| item.get(0)).map_err(|err| format!("Falha ao reler nota migrada: {err}"))?;
            conn.execute("INSERT OR REPLACE INTO manifestador_legacy_nf_map (legacy_codigo, nota_id, created_at) VALUES (?1, ?2, ?3)", params![legacy.codigo, id, Utc::now().to_rfc3339()])
                .map_err(|err| format!("Falha ao gravar mapa da nota legada: {err}"))?;
            id
        };
        migrate_legacy_items(conn, legacy.codigo, note_id)?;
        migrate_legacy_logs(conn, legacy.codigo, note_id)?;
    }
    Ok(())
}

fn migrate_legacy_items(conn: &Connection, legacy_nf: i64, note_id: i64) -> Result<(), String> {
    if !table_exists(conn, "ITENS")? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT ITEM, CPROD, XPROD, NCM, CFOP, UCOM, QCOM, UTRIB, QTRIB, VUNTRIB, ORIG, CSOSN, ST, VBCICMS, PICMS, VICMS, VICMSST, VBCICMSST, VIPI, PIPI FROM ITENS WHERE NF = ?1 ORDER BY ITEM").map_err(|err| format!("Falha ao preparar itens Delphi: {err}"))?;
    let rows = stmt
        .query_map([legacy_nf], |row| {
            Ok(ParsedItem {
                item: row_i64(row, 0)?,
                cprod: row_text(row, 1)?,
                xprod: row_text(row, 2)?,
                ncm: row_text(row, 3)?,
                cfop: row_text(row, 4)?,
                ucom: row_text(row, 5)?,
                qcom: row_f64(row, 6)?,
                utrib: row_text(row, 7)?,
                qtrib: row_f64(row, 8)?,
                vuntrib: row_f64(row, 9)?,
                orig: row_text(row, 10)?,
                csosn: row_text(row, 11)?,
                st: row_text(row, 12)?,
                vbcicms: row_f64(row, 13)?,
                picms: row_f64(row, 14)?,
                vicms: row_f64(row, 15)?,
                vicmsst: row_f64(row, 16)?,
                vbcicmsst: row_f64(row, 17)?,
                vipi: row_f64(row, 18)?,
                pipi: row_f64(row, 19)?,
            })
        })
        .map_err(|err| format!("Falha ao ler itens Delphi: {err}"))?;
    for row in rows {
        insert_item(
            conn,
            note_id,
            &row.map_err(|err| format!("Falha ao mapear item Delphi: {err}"))?,
        )?;
    }
    Ok(())
}

fn migrate_legacy_logs(conn: &Connection, legacy_nf: i64, note_id: i64) -> Result<(), String> {
    if !table_exists(conn, "NF_LOG")? {
        return Ok(());
    }
    let mut stmt = conn
        .prepare(
            "SELECT OPERACAO, DATAHORA, HISTORICO FROM NF_LOG WHERE ID_NF = ?1 ORDER BY CODIGO",
        )
        .map_err(|err| format!("Falha ao preparar histórico Delphi: {err}"))?;
    let rows = stmt
        .query_map([legacy_nf], |row| {
            Ok((row_i64(row, 0)?, row_text(row, 1)?, row_text(row, 2)?))
        })
        .map_err(|err| format!("Falha ao ler histórico Delphi: {err}"))?;
    for row in rows {
        let (operation, date, history) =
            row.map_err(|err| format!("Falha ao mapear histórico Delphi: {err}"))?;
        conn.execute("INSERT OR IGNORE INTO manifestador_nf_log (nota_id, operacao, datahora, historico) VALUES (?1, ?2, ?3, ?4)", params![note_id, operation, date, history]).map_err(|err| format!("Falha ao migrar histórico Delphi: {err}"))?;
    }
    Ok(())
}

fn insert_item(conn: &Connection, note_id: i64, item: &ParsedItem) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO manifestador_itens (nota_id, item, cprod, xprod, ncm, cfop, ucom, qcom, utrib, qtrib, vuntrib, orig, csosn, st, vbcicms, picms, vicms, vicmsst, vbcicmsst, vipi, pipi) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![note_id, item.item, empty_to_none(&item.cprod), empty_to_none(&item.xprod), empty_to_none(&item.ncm), empty_to_none(&item.cfop), empty_to_none(&item.ucom), item.qcom, empty_to_none(&item.utrib), item.qtrib, item.vuntrib, empty_to_none(&item.orig), empty_to_none(&item.csosn), empty_to_none(&item.st), item.vbcicms, item.picms, item.vicms, item.vicmsst, item.vbcicmsst, item.vipi, item.pipi],
    )
    .map_err(|err| format!("Falha ao gravar item da NF-e: {err}"))?;
    Ok(())
}

fn encrypted_optional(key: &str, value: &str) -> Option<String> {
    empty_to_none(value).map(|item| encrypt_text(key, &item))
}

fn empty_to_none(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}
fn default_text(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}
fn bool_from_legacy(value: &str) -> i64 {
    matches!(
        value.trim().to_ascii_uppercase().as_str(),
        "S" | "SIM" | "1" | "Y" | "YES"
    )
    .then_some(1)
    .unwrap_or(0)
}
fn environment_from_legacy(value: &str) -> String {
    if matches!(
        value.trim(),
        "2" | "H" | "h" | "homologacao" | "homologação"
    ) {
        "homologation".to_string()
    } else {
        "production".to_string()
    }
}

fn parse_decimal(value: Option<&String>) -> f64 {
    value
        .and_then(|item| item.trim().replace(',', ".").parse::<f64>().ok())
        .unwrap_or_default()
}
fn local_name(value: &[u8]) -> String {
    String::from_utf8_lossy(value)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_string()
}
fn path_value(values: &HashMap<String, String>, suffixes: &[&str]) -> String {
    suffixes
        .iter()
        .find_map(|suffix| {
            values
                .iter()
                .find(|(key, _)| key.ends_with(suffix))
                .map(|(_, value)| value.clone())
        })
        .unwrap_or_default()
}

pub fn parse_nfe_xml(xml: &str) -> Result<ParsedNfe, String> {
    if xml.trim().is_empty() {
        return Err("O XML da NF-e está vazio.".to_string());
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut path: Vec<String> = Vec::new();
    let mut values: HashMap<String, String> = HashMap::new();
    let mut items: Vec<ParsedItem> = Vec::new();
    let mut current_item: Option<ParsedItem> = None;
    let mut chave = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = local_name(event.name().as_ref());
                if tag == "infNFe" {
                    for attr in event.attributes().with_checks(false) {
                        let attr =
                            attr.map_err(|err| format!("Falha ao ler atributo da NF-e: {err}"))?;
                        if local_name(attr.key.as_ref()) == "Id" {
                            chave = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .map_err(|err| format!("Falha ao ler chave da NF-e: {err}"))?
                                .trim_start_matches("NFe")
                                .to_string();
                        }
                    }
                }
                if tag == "det" {
                    let mut item_number = (items.len() + 1) as i64;
                    for attr in event.attributes().with_checks(false) {
                        let attr =
                            attr.map_err(|err| format!("Falha ao ler item da NF-e: {err}"))?;
                        if local_name(attr.key.as_ref()) == "nItem" {
                            item_number = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .ok()
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(item_number);
                        }
                    }
                    current_item = Some(ParsedItem {
                        item: item_number,
                        ..ParsedItem::default()
                    });
                }
                path.push(tag);
            }
            Ok(Event::Empty(event)) => {
                let tag = local_name(event.name().as_ref());
                if tag == "infNFe" {
                    for attr in event.attributes().with_checks(false) {
                        let attr =
                            attr.map_err(|err| format!("Falha ao ler chave da NF-e: {err}"))?;
                        if local_name(attr.key.as_ref()) == "Id" {
                            chave = attr
                                .normalized_value(XmlVersion::Implicit1_0)
                                .unwrap_or_default()
                                .trim_start_matches("NFe")
                                .to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(event)) if !path.is_empty() => {
                let text = event
                    .decode()
                    .map_err(|err| format!("Falha ao ler texto do XML da NF-e: {err}"))?
                    .into_owned();
                if let Some(last) = path.last() {
                    let key = path.join("/");
                    values.insert(key, text.clone());
                    if let Some(item) = current_item.as_mut() {
                        assign_item_value(item, last, &text, &path);
                    }
                }
            }
            Ok(Event::CData(event)) if !path.is_empty() => {
                values.insert(
                    path.join("/"),
                    String::from_utf8_lossy(event.as_ref()).to_string(),
                );
            }
            Ok(Event::End(event)) => {
                let tag = local_name(event.name().as_ref());
                if tag == "det" {
                    if let Some(item) = current_item.take() {
                        items.push(item);
                    }
                }
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("XML da NF-e inválido: {err}")),
            _ => {}
        }
    }
    if chave.is_empty() {
        chave = path_value(&values, &["/chNFe"]).trim().to_string();
    }
    if chave.is_empty() {
        return Err("O XML não contém a chave de acesso da NF-e.".to_string());
    }
    let ambiente = match path_value(&values, &["/tpAmb"]).trim() {
        "2" => "homologation",
        _ => "production",
    }
    .to_string();
    let mut result = ParsedNfe {
        chave,
        numero: path_value(&values, &["/ide/nNF"])
            .parse()
            .unwrap_or_default(),
        serie: path_value(&values, &["/ide/serie"])
            .parse()
            .unwrap_or_default(),
        cnpj: normalize_document(&path_value(&values, &["/emit/CNPJ", "/emit/CPF"])),
        nome: path_value(&values, &["/emit/xNome"]),
        ie: path_value(&values, &["/emit/IE"]),
        emissao: path_value(&values, &["/ide/dhEmi", "/ide/dEmi"]),
        valor: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vNF"]))),
        tipo: "NFe".to_string(),
        ambiente,
        total_produtos: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vProd"]))),
        valor_frete: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vFrete"]))),
        valor_despesas: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vOutro"]))),
        valor_desconto: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vDesc"]))),
        valor_ipi: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vIPI"]))),
        valor_icms: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vICMS"]))),
        valor_icms_st: parse_decimal(Some(&path_value(&values, &["/total/ICMSTot/vST"]))),
        endereco: path_value(&values, &["/enderEmit/xLgr"]),
        endereco_numero: path_value(&values, &["/enderEmit/nro"]),
        endereco_complemento: path_value(&values, &["/enderEmit/xCpl"]),
        endereco_bairro: path_value(&values, &["/enderEmit/xBairro"]),
        endereco_cep: path_value(&values, &["/enderEmit/CEP"]),
        endereco_cidade: path_value(&values, &["/enderEmit/xMun"]),
        endereco_codmunicipio: path_value(&values, &["/enderEmit/cMun"]),
        telefone: path_value(&values, &["/enderEmit/fone"]),
        protocolo: path_value(&values, &["/protNFe/infProt/nProt"]),
        items,
    };
    result.items.sort_by_key(|item| item.item);
    Ok(result)
}

/// Decodifica o conteúdo `docZip` utilizado pelo retorno DistribuicaoDFe.
/// O fallback para XML puro facilita o uso com adaptadores locais de teste.
pub fn decode_doczip(value: &str) -> Result<String, String> {
    let bytes = BASE64_STANDARD
        .decode(value.trim())
        .map_err(|err| format!("Falha ao decodificar docZip: {err}"))?;
    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut xml = String::new();
    match decoder.read_to_string(&mut xml) {
        Ok(_) if !xml.trim().is_empty() => Ok(xml),
        _ => String::from_utf8(bytes).map_err(|err| format!("docZip não contém XML válido: {err}")),
    }
}

fn assign_item_value(item: &mut ParsedItem, tag: &str, value: &str, path: &[String]) {
    if !path
        .iter()
        .any(|item| item == "prod" || item == "ICMS" || item == "IPI")
    {
        return;
    }
    match tag {
        "cProd" => item.cprod = value.to_string(),
        "xProd" => item.xprod = value.to_string(),
        "NCM" => item.ncm = value.to_string(),
        "CFOP" => item.cfop = value.to_string(),
        "uCom" => item.ucom = value.to_string(),
        "qCom" => item.qcom = parse_decimal(Some(&value.to_string())),
        "uTrib" => item.utrib = value.to_string(),
        "qTrib" => item.qtrib = parse_decimal(Some(&value.to_string())),
        "vUnTrib" => item.vuntrib = parse_decimal(Some(&value.to_string())),
        "orig" => item.orig = value.to_string(),
        "CSOSN" | "CST" if item.csosn.is_empty() => item.csosn = value.to_string(),
        "vBC" => item.vbcicms = parse_decimal(Some(&value.to_string())),
        "pICMS" => item.picms = parse_decimal(Some(&value.to_string())),
        "vICMS" => item.vicms = parse_decimal(Some(&value.to_string())),
        "vICMSST" => item.vicmsst = parse_decimal(Some(&value.to_string())),
        "vBCST" => item.vbcicmsst = parse_decimal(Some(&value.to_string())),
        "vIPI" => item.vipi = parse_decimal(Some(&value.to_string())),
        "pIPI" => item.pipi = parse_decimal(Some(&value.to_string())),
        _ => {}
    }
}

pub fn import_xml(
    conn: &mut Connection,
    empresa_id: i64,
    xml: &str,
    nsu: i64,
    ambiente_hint: Option<&str>,
) -> Result<Value, String> {
    let parsed = parse_nfe_xml(xml)?;
    let ambiente = ambiente_hint
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&parsed.ambiente)
        .to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO manifestador_notas (empresa_id, numero, serie, cnpj, nome, emissao, valor, tipo, situacao, chave, ie, protocolo, data_recebimento, ambiente, nsu, total_produtos, valor_frete, valor_despesas, valor_desconto, valor_ipi, valor_icms, valor_icms_st, endereco, endereco_numero, endereco_complemento, endereco_bairro, endereco_cep, endereco_cidade, endereco_codmunicipio, telefone, xml_content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'baixada', ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32) ON CONFLICT(empresa_id, chave, ambiente) DO UPDATE SET numero=excluded.numero, serie=excluded.serie, cnpj=excluded.cnpj, nome=excluded.nome, emissao=excluded.emissao, valor=excluded.valor, tipo=excluded.tipo, ie=excluded.ie, protocolo=excluded.protocolo, data_recebimento=excluded.data_recebimento, nsu=CASE WHEN excluded.nsu > 0 THEN excluded.nsu ELSE manifestador_notas.nsu END, total_produtos=excluded.total_produtos, valor_frete=excluded.valor_frete, valor_despesas=excluded.valor_despesas, valor_desconto=excluded.valor_desconto, valor_ipi=excluded.valor_ipi, valor_icms=excluded.valor_icms, valor_icms_st=excluded.valor_icms_st, endereco=excluded.endereco, endereco_numero=excluded.endereco_numero, endereco_complemento=excluded.endereco_complemento, endereco_bairro=excluded.endereco_bairro, endereco_cep=excluded.endereco_cep, endereco_cidade=excluded.endereco_cidade, endereco_codmunicipio=excluded.endereco_codmunicipio, telefone=excluded.telefone, xml_content=excluded.xml_content, xml_path=NULL, updated_at=excluded.updated_at",
        params![empresa_id, parsed.numero, parsed.serie, parsed.cnpj.clone(), parsed.nome.clone(), empty_to_none(&parsed.emissao), parsed.valor, parsed.tipo.clone(), parsed.chave.clone(), empty_to_none(&parsed.ie), empty_to_none(&parsed.protocolo), now.clone(), ambiente.clone(), nsu, parsed.total_produtos, parsed.valor_frete, parsed.valor_despesas, parsed.valor_desconto, parsed.valor_ipi, parsed.valor_icms, parsed.valor_icms_st, empty_to_none(&parsed.endereco), empty_to_none(&parsed.endereco_numero), empty_to_none(&parsed.endereco_complemento), empty_to_none(&parsed.endereco_bairro), empty_to_none(&parsed.endereco_cep), empty_to_none(&parsed.endereco_cidade), empty_to_none(&parsed.endereco_codmunicipio), empty_to_none(&parsed.telefone), xml, now.clone(), now],
    ).map_err(|err| format!("Falha ao importar NF-e: {err}"))?;
    let note_id: i64 = conn.query_row("SELECT id FROM manifestador_notas WHERE empresa_id = ?1 AND chave = ?2 AND ambiente = ?3", params![empresa_id, parsed.chave, ambiente], |row| row.get(0)).map_err(|err| format!("Falha ao localizar NF-e importada: {err}"))?;
    conn.execute(
        "DELETE FROM manifestador_itens WHERE nota_id = ?1",
        [note_id],
    )
    .map_err(|err| format!("Falha ao substituir itens da NF-e: {err}"))?;
    for item in &parsed.items {
        insert_item(conn, note_id, item)?;
    }
    add_log(conn, note_id, 4, "XML importado no Manifestador Tauri/Rust")?;
    note_json(conn, note_id)
}

/// Persiste o resumo retornado por DistribuicaoDFe quando o provedor ainda não
/// entregou o XML completo. O registro pode ser enriquecido depois por
/// `import_xml` ou por `download_note`, sem perder manifestação, download ou
/// histórico já existentes.
pub fn import_summary(
    conn: &mut Connection,
    empresa_id: i64,
    object: &Map<String, Value>,
    nsu_hint: i64,
    ambiente_hint: Option<&str>,
) -> Result<Value, String> {
    let chave = object_string(object, &["chave", "chDFe", "chNFe", "CHAVE"]);
    if chave.is_empty() {
        return Err("O resumo recebido não contém a chave de acesso da NF-e.".to_string());
    }
    let ambiente_raw = object_string(object, &["ambiente", "tpAmb", "AMBIENTE"]);
    let ambiente = ambiente_hint
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if ambiente_raw.trim() == "2" {
                "homologation"
            } else {
                "production"
            }
            .to_string()
        });
    let tipo_raw = object_string(object, &["tipo", "tpNF", "TIPO"]);
    let tipo = match tipo_raw.trim() {
        "0" => "Entrada".to_string(),
        "1" => "Saída".to_string(),
        "" => "NFe".to_string(),
        value => value.to_string(),
    };
    let situacao_raw = object_string(object, &["situacao", "cSitDFe", "SITUACAO"]);
    let situacao = match situacao_raw.trim() {
        "1" => "Autorizado",
        "2" => "Denegado",
        "3" => "Cancelado",
        "4" => "Encerrado",
        "" => "recebida",
        value => value,
    };
    let nsu = if nsu_hint > 0 {
        nsu_hint
    } else {
        object_i64(object, &["nsu", "NSU"])
    };
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO manifestador_notas (empresa_id, numero, serie, cnpj, nome, emissao, valor, tipo, situacao, chave, ie, protocolo, data_recebimento, ambiente, nsu, visivel, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1, ?16, ?16) ON CONFLICT(empresa_id, chave, ambiente) DO UPDATE SET numero=excluded.numero, serie=excluded.serie, cnpj=excluded.cnpj, nome=excluded.nome, emissao=excluded.emissao, valor=excluded.valor, tipo=excluded.tipo, situacao=excluded.situacao, ie=excluded.ie, protocolo=excluded.protocolo, data_recebimento=excluded.data_recebimento, nsu=CASE WHEN excluded.nsu > 0 THEN excluded.nsu ELSE manifestador_notas.nsu END, updated_at=excluded.updated_at",
        params![
            empresa_id,
            object_i64(object, &["numero", "nNF", "NUMERO"]),
            object_i64(object, &["serie", "SERIE"]),
            normalize_document(&object_string(object, &["cnpj", "CNPJCPF", "CNPJ", "CPF"])),
            object_string(object, &["nome", "xNome", "NOME"]),
            empty_to_none(&object_string(object, &["emissao", "dhEmi", "dEmi", "EMISSAO"])),
            object_f64(object, &["valor", "vNF", "VALOR"]),
            tipo,
            situacao,
            chave.clone(),
            empty_to_none(&object_string(object, &["ie", "IE"])),
            empty_to_none(&object_string(object, &["protocolo", "nProt", "PROTOCOLO"])),
            empty_to_none(&object_string(object, &["data_recebimento", "dhRecbto", "DATAREC"])),
            ambiente.clone(),
            nsu,
            now,
        ],
    )
    .map_err(|err| format!("Falha ao importar resumo da NF-e: {err}"))?;
    let note_id: i64 = conn
        .query_row(
            "SELECT id FROM manifestador_notas WHERE empresa_id = ?1 AND chave = ?2 AND ambiente = ?3",
            params![empresa_id, chave, ambiente],
            |row| row.get(0),
        )
        .map_err(|err| format!("Falha ao localizar resumo importado: {err}"))?;
    add_log(
        conn,
        note_id,
        1,
        "Resumo de NF-e recebido pelo adaptador externo",
    )?;
    note_json(conn, note_id)
}

pub fn note_json(conn: &Connection, id: i64) -> Result<Value, String> {
    let mut result = conn
        .query_row(
            "SELECT * FROM manifestador_notas WHERE id = ?1",
            [id],
            |row| Ok(Value::Object(row_to_json_map(row)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao consultar NF-e: {err}"))?
        .ok_or_else(|| "NF-e não encontrada.".to_string())?;
    if let Value::Object(map) = &mut result {
        let mut stmt = conn
            .prepare("SELECT * FROM manifestador_itens WHERE nota_id = ?1 ORDER BY item")
            .map_err(|err| format!("Falha ao preparar itens da NF-e: {err}"))?;
        let items = stmt
            .query_map([id], row_to_json_map)
            .map_err(|err| format!("Falha ao consultar itens da NF-e: {err}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Falha ao mapear itens da NF-e: {err}"))?
            .into_iter()
            .map(Value::Object)
            .collect::<Vec<_>>();
        map.insert("items".to_string(), Value::Array(items));
    }
    Ok(result)
}

pub fn add_log(
    conn: &Connection,
    note_id: i64,
    operation: i64,
    history: &str,
) -> Result<(), String> {
    conn.execute("INSERT INTO manifestador_nf_log (nota_id, operacao, datahora, historico) VALUES (?1, ?2, ?3, ?4)", params![note_id, operation, Utc::now().to_rfc3339(), history]).map_err(|err| format!("Falha ao gravar histórico da NF-e: {err}"))?;
    Ok(())
}

pub fn list_notes(
    conn: &Connection,
    filters: &Map<String, Value>,
) -> Result<Vec<Map<String, Value>>, String> {
    let company_id = value_i64(
        filters
            .get("empresa_id")
            .or_else(|| filters.get("empresaId")),
    );
    let search = value_string(filters.get("search"));
    let status = value_string(filters.get("status"));
    let manifestation = value_string(filters.get("manifestacao"));
    let date_from = value_string(filters.get("data_inicial"));
    let date_to = value_string(filters.get("data_final"));
    let mut sql = "SELECT * FROM manifestador_notas WHERE visivel = 1".to_string();
    let mut values: Vec<rusqlite::types::Value> = Vec::new();
    if let Some(company_id) = company_id {
        sql.push_str(" AND empresa_id = ?");
        values.push(rusqlite::types::Value::Integer(company_id));
    }
    if !search.is_empty() {
        sql.push_str(
            " AND (chave LIKE ? OR nome LIKE ? OR cnpj LIKE ? OR CAST(numero AS TEXT) LIKE ?)",
        );
        let text = rusqlite::types::Value::Text(format!("%{search}%"));
        values.extend([text.clone(), text.clone(), text.clone(), text]);
    }
    if !status.is_empty() {
        sql.push_str(" AND situacao = ?");
        values.push(rusqlite::types::Value::Text(status));
    }
    if !manifestation.is_empty() {
        sql.push_str(" AND manifestacao = ?");
        values.push(rusqlite::types::Value::Text(manifestation));
    }
    if !date_from.is_empty() {
        sql.push_str(" AND COALESCE(emissao, data_recebimento, created_at) >= ?");
        values.push(rusqlite::types::Value::Text(date_from));
    }
    if !date_to.is_empty() {
        sql.push_str(" AND COALESCE(emissao, data_recebimento, created_at) < ?");
        values.push(rusqlite::types::Value::Text(date_to));
    }
    sql.push_str(
        " ORDER BY COALESCE(emissao, data_recebimento, created_at) DESC, id DESC LIMIT 1000",
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Falha ao preparar notas: {err}"))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(values.iter()), row_to_json_map)
        .map_err(|err| format!("Falha ao listar notas: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear notas: {err}"))
}

pub fn list_logs(conn: &Connection, note_id: i64) -> Result<Vec<Map<String, Value>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT * FROM manifestador_nf_log WHERE nota_id = ?1 ORDER BY datahora DESC, id DESC",
        )
        .map_err(|err| format!("Falha ao preparar histórico: {err}"))?;
    let rows = stmt
        .query_map([note_id], row_to_json_map)
        .map_err(|err| format!("Falha ao consultar histórico: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("Falha ao mapear histórico: {err}"))
}

pub fn summary(conn: &Connection, empresa_id: Option<i64>) -> Result<Value, String> {
    let condition = empresa_id.map(|_| " WHERE empresa_id = ?1").unwrap_or("");
    let total: i64 = if let Some(id) = empresa_id {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM manifestador_notas{condition}"),
            [id],
            |row| row.get(0),
        )
    } else {
        conn.query_row("SELECT COUNT(*) FROM manifestador_notas", [], |row| {
            row.get(0)
        })
    }
    .map_err(|err| format!("Falha ao contar notas: {err}"))?;
    let downloaded: i64 = if let Some(id) = empresa_id {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM manifestador_notas{condition} AND download = 1"),
            [id],
            |row| row.get(0),
        )
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM manifestador_notas WHERE download = 1",
            [],
            |row| row.get(0),
        )
    }
    .map_err(|err| format!("Falha ao contar downloads: {err}"))?;
    let manifested: i64 = if let Some(id) = empresa_id { conn.query_row(&format!("SELECT COUNT(*) FROM manifestador_notas{condition} AND manifestacao NOT IN ('N', '')"), [id], |row| row.get(0)) } else { conn.query_row("SELECT COUNT(*) FROM manifestador_notas WHERE manifestacao NOT IN ('N', '')", [], |row| row.get(0)) }.map_err(|err| format!("Falha ao contar manifestações: {err}"))?;
    Ok(
        json!({"total": total, "downloaded": downloaded, "manifested": manifested, "pending": total - manifested}),
    )
}

pub fn get_config(conn: &Connection, empresa_id: i64) -> Result<Value, String> {
    ensure_operation_config(conn, empresa_id)?;
    let mut result = conn
        .query_row(
            "SELECT * FROM manifestador_empresa_operacao WHERE empresa_id = ?1",
            [empresa_id],
            |row| Ok(Value::Object(row_to_json_map(row)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar configuração do Manifestador: {err}"))?
        .ok_or_else(|| "Configuração do Manifestador não encontrada.".to_string())?;
    if let Value::Object(map) = &mut result {
        let password_configured = map
            .remove("smtp_senha_encrypted")
            .and_then(|v| v.as_str().map(|s| !s.trim().is_empty()))
            .unwrap_or(false);
        map.insert(
            "smtp_password_configured".to_string(),
            Value::Bool(password_configured),
        );
    }
    Ok(result)
}

pub fn save_config(
    conn: &Connection,
    empresa_id: i64,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    ensure_operation_config(conn, empresa_id)?;
    let existing_password: Option<String> = conn
        .query_row(
            "SELECT smtp_senha_encrypted FROM manifestador_empresa_operacao WHERE empresa_id = ?1",
            [empresa_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar senha SMTP: {err}"))?
        .flatten();
    let password = value_string(payload.get("smtp_password"));
    let password = if password.is_empty() {
        value_string(payload.get("smtp_senha"))
    } else {
        password
    };
    let encrypted_password = if password.is_empty() {
        existing_password
    } else {
        Some(encrypt_text(&machine_key(), &password))
    };
    conn.execute("UPDATE manifestador_empresa_operacao SET verificar_automatico = ?1, download_automatico = ?2, timer_minutos = ?3, smtp_host = ?4, smtp_conta = ?5, smtp_remetente = ?6, smtp_email = ?7, smtp_senha_encrypted = ?8, smtp_porta = ?9, sefaz_endpoint = ?10, sefaz_service_url = ?11, sefaz_timeout_seconds = ?12, updated_at = ?13 WHERE empresa_id = ?14", params![value_bool(payload.get("verificar_automatico"), true), value_bool(payload.get("download_automatico"), true), value_i64(payload.get("timer_minutos")).unwrap_or(60).max(1), empty_to_none(&value_string(payload.get("smtp_host"))), empty_to_none(&value_string(payload.get("smtp_conta"))), empty_to_none(&value_string(payload.get("smtp_remetente"))), empty_to_none(&value_string(payload.get("smtp_email"))), encrypted_password, value_i64(payload.get("smtp_porta")).unwrap_or(587).max(1), empty_to_none(&value_string(payload.get("sefaz_endpoint"))), empty_to_none(&value_string(payload.get("sefaz_service_url"))), value_i64(payload.get("sefaz_timeout_seconds")).unwrap_or(DEFAULT_TIMEOUT_SECONDS).clamp(5, 300), Utc::now().to_rfc3339(), empresa_id]).map_err(|err| format!("Falha ao salvar configuração do Manifestador: {err}"))?;
    get_config(conn, empresa_id)
}

pub fn get_company_config(conn: &Connection, empresa_id: i64) -> Result<Value, String> {
    conn.execute("INSERT OR IGNORE INTO manifestador_empresa_config (empresa_id, created_at, updated_at) SELECT id, ?2, ?2 FROM empresas WHERE id = ?1", params![empresa_id, Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
    let mut result = conn
        .query_row(
            "SELECT * FROM manifestador_empresa_config WHERE empresa_id = ?1",
            [empresa_id],
            |row| Ok(Value::Object(row_to_json_map(row)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar configuração fiscal: {err}"))?
        .ok_or_else(|| "Configuração fiscal da empresa não encontrada.".to_string())?;
    if let Value::Object(map) = &mut result {
        map.remove("certificado_password_encrypted");
        map.insert(
            "certificado_password_configured".to_string(),
            Value::Bool(has_secret(
                conn,
                "certificado_password_encrypted",
                Some(empresa_id),
            )?),
        );
    }
    Ok(result)
}

pub fn save_company_config(
    conn: &Connection,
    empresa_id: i64,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let conn = &tx;
    if let Some(operation) = payload.get("operation_config").and_then(Value::as_object) {
        save_config(conn, empresa_id, operation)?;
    }
    let current: Option<String> = conn.query_row("SELECT certificado_password_encrypted FROM manifestador_empresa_config WHERE empresa_id = ?1", [empresa_id], |row| row.get(0)).optional().map_err(|err| format!("Falha ao carregar senha do certificado: {err}"))?.flatten();
    let password = value_string(payload.get("certificado_password"));
    let password = if password.is_empty() {
        current
    } else {
        Some(encrypt_text(
            &format!("{}:{empresa_id}", machine_key()),
            &password,
        ))
    };
    let now = Utc::now().to_rfc3339();
    conn.execute("INSERT INTO manifestador_empresa_config (empresa_id, certificado_path, certificado_password_encrypted, uf, xml_path, numero_serie, copia_xml_path, resposta_path, ambiente, nsu, data_manifestacao, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(?10, '0'), ?11, ?12, ?12) ON CONFLICT(empresa_id) DO UPDATE SET certificado_path=excluded.certificado_path, certificado_password_encrypted=excluded.certificado_password_encrypted, uf=excluded.uf, xml_path=excluded.xml_path, numero_serie=excluded.numero_serie, copia_xml_path=excluded.copia_xml_path, resposta_path=excluded.resposta_path, ambiente=excluded.ambiente, nsu=COALESCE(?10, manifestador_empresa_config.nsu), data_manifestacao=COALESCE(?11, manifestador_empresa_config.data_manifestacao), updated_at=excluded.updated_at", params![empresa_id, empty_to_none(&value_string(payload.get("certificado_path"))), password, empty_to_none(&value_string(payload.get("uf"))), empty_to_none(&value_string(payload.get("xml_path"))), empty_to_none(&value_string(payload.get("numero_serie"))), empty_to_none(&value_string(payload.get("copia_xml_path"))), empty_to_none(&value_string(payload.get("resposta_path"))), default_text(&value_string(payload.get("ambiente")), "production"), empty_to_none(&value_string(payload.get("nsu"))), empty_to_none(&value_string(payload.get("data_manifestacao"))), now]).map_err(|err| format!("Falha ao salvar configuração fiscal: {err}"))?;
    let result = get_company_config(conn, empresa_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}

fn has_secret(conn: &Connection, column: &str, empresa_id: Option<i64>) -> Result<bool, String> {
    let (sql, parameter) = if let Some(empresa_id) = empresa_id {
        (format!("SELECT COALESCE(TRIM({column}), '') <> '' FROM manifestador_empresa_config WHERE empresa_id = ?1"), Some(empresa_id))
    } else {
        (
            format!(
                "SELECT COALESCE(TRIM({column}), '') <> '' FROM manifestador_config WHERE id = 1"
            ),
            None,
        )
    };
    if let Some(empresa_id) = parameter {
        Ok(conn
            .query_row(&sql, [empresa_id], |row| row.get::<_, bool>(0))
            .optional()
            .map_err(|err| format!("Falha ao verificar segredo configurado: {err}"))?
            .unwrap_or(false))
    } else {
        Ok(conn
            .query_row(&sql, [], |row| row.get::<_, bool>(0))
            .optional()
            .map_err(|err| format!("Falha ao verificar segredo configurado: {err}"))?
            .unwrap_or(false))
    }
}

fn value_string(value: Option<&Value>) -> String {
    value
        .and_then(|value| match value {
            Value::String(text) => Some(text.trim().to_string()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(if *flag {
                "1".to_string()
            } else {
                "0".to_string()
            }),
            _ => None,
        })
        .unwrap_or_default()
}
fn value_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    })
}
fn value_bool(value: Option<&Value>, default: bool) -> i64 {
    value
        .map(|value| match value {
            Value::Bool(flag) => *flag,
            Value::Number(number) => number.as_i64().unwrap_or_default() != 0,
            Value::String(text) => matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "sim" | "yes"
            ),
            _ => default,
        })
        .unwrap_or(default) as i64
}

fn object_value<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    if let Some(value) = keys.iter().find_map(|key| object.get(*key)) {
        return Some(value);
    }
    object
        .get("resDFe")
        .and_then(Value::as_object)
        .and_then(|summary| keys.iter().find_map(|key| summary.get(*key)))
}

fn object_string(object: &Map<String, Value>, keys: &[&str]) -> String {
    value_string(object_value(object, keys))
}

fn object_i64(object: &Map<String, Value>, keys: &[&str]) -> i64 {
    value_i64(object_value(object, keys)).unwrap_or_default()
}

fn object_f64(object: &Map<String, Value>, keys: &[&str]) -> f64 {
    object_value(object, keys)
        .and_then(|value| match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.trim().replace(',', ".").parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or_default()
}

pub fn update_manifestation(
    conn: &Connection,
    note_id: i64,
    event: &str,
    justification: &str,
    protocol: Option<&str>,
) -> Result<Value, String> {
    let normalized = event.trim().to_ascii_lowercase();
    let (code, label) = match normalized.as_str() {
        "confirmacao" | "confirmacao_da_operacao" | "0" => ("0", "Confirmação da operação"),
        "ciencia" | "ciencia_da_operacao" | "1" => ("1", "Ciência da operação"),
        "nao_realizada" | "operacao_nao_realizada" | "2" => {
            if justification.trim().chars().count() < 15 {
                return Err("A justificativa de operação não realizada deve possuir ao menos 15 caracteres.".to_string());
            }
            ("2", "Operação não realizada")
        }
        "desconhecimento" | "desconhecimento_da_operacao" | "3" => {
            ("3", "Desconhecimento da operação")
        }
        _ => return Err("Tipo de manifestação inválido.".to_string()),
    };
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE manifestador_notas SET manifestacao = ?1, situacao = 'manifestada_local', protocolo = COALESCE(?2, protocolo), updated_at = ?3 WHERE id = ?4", params![code, protocol.filter(|value| !value.trim().is_empty()), now, note_id]).map_err(|err| format!("Falha ao registrar manifestação: {err}"))?;
    add_log(
        conn,
        note_id,
        3,
        &format!("{label} registrada localmente pela aplicação Tauri/Rust"),
    )?;
    note_json(conn, note_id)
}

pub fn download_note(
    conn: &Connection,
    data_dir: &Path,
    empresa_id: i64,
    note_id: i64,
    xml_override: Option<&str>,
) -> Result<Value, String> {
    let (chave, xml_content, xml_path): (String, Option<String>, Option<String>) = conn.query_row("SELECT chave, xml_content, xml_path FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2", params![note_id, empresa_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(|err| format!("Falha ao localizar NF-e para download: {err}"))?.ok_or_else(|| "NF-e não encontrada para a empresa ativa.".to_string())?;
    let xml = if let Some(xml) = xml_override.filter(|value| !value.trim().is_empty()) {
        xml.to_string()
    } else if let Some(xml) = xml_content {
        xml
    } else if let Some(path) = xml_path {
        fs::read_to_string(path).map_err(|err| format!("Falha ao ler XML vinculado: {err}"))?
    } else {
        return Err("Esta NF-e ainda não possui XML completo para download.".to_string());
    };
    let configured_dir: Option<String> = conn.query_row("SELECT COALESCE(copia_xml_path, xml_path) FROM manifestador_empresa_config WHERE empresa_id = ?1", [empresa_id], |row| row.get(0)).optional().map_err(|err| format!("Falha ao consultar diretório de XML: {err}"))?.flatten();
    let target_dir = configured_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("xml").join(empresa_id.to_string()));
    fs::create_dir_all(&target_dir)
        .map_err(|err| format!("Falha ao criar diretório de XML: {err}"))?;
    let target = target_dir.join(format!("{chave}.xml"));
    fs::write(&target, xml.as_bytes())
        .map_err(|err| format!("Falha ao salvar XML baixado: {err}"))?;
    conn.execute("UPDATE manifestador_notas SET download = 1, download_path = ?1, xml_content = ?2, updated_at = ?3 WHERE id = ?4", params![target.to_string_lossy().to_string(), xml, Utc::now().to_rfc3339(), note_id]).map_err(|err| format!("Falha ao atualizar situação de download: {err}"))?;
    add_log(
        conn,
        note_id,
        2,
        "XML completo salvo no diretório configurado",
    )?;
    Ok(
        json!({"note": note_json(conn, note_id)?, "path": target.to_string_lossy(), "message": "XML salvo com sucesso."}),
    )
}

pub fn export_zip(
    conn: &Connection,
    data_dir: &Path,
    empresa_id: Option<i64>,
    filters: &Map<String, Value>,
) -> Result<Value, String> {
    let mut filter = filters.clone();
    if let Some(id) = empresa_id {
        filter.insert("empresa_id".to_string(), Value::from(id));
    }
    let notes = list_notes(conn, &filter)?;
    let export_dir = data_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|err| format!("Falha ao criar diretório de exportação: {err}"))?;
    let filename = format!(
        "manifestador-nfe-{}.zip",
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    let target = export_dir.join(filename);
    let mut entries = Vec::new();
    for note in notes {
        let id = note.get("id").and_then(Value::as_i64).unwrap_or_default();
        let chave = value_string(note.get("chave"));
        let xml: Option<String> = conn
            .query_row(
                "SELECT xml_content FROM manifestador_notas WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| format!("Falha ao carregar XML para exportação: {err}"))?
            .flatten();
        if let Some(xml) = xml {
            entries.push((
                format!(
                    "{}.xml",
                    if chave.is_empty() {
                        id.to_string()
                    } else {
                        chave
                    }
                ),
                xml.into_bytes(),
            ));
        }
    }
    if entries.is_empty() {
        return Err("Nenhum XML completo está disponível para exportação.".to_string());
    }
    let count = entries.len();
    write_stored_zip(&target, entries)?;
    Ok(
        json!({"path": target.to_string_lossy(), "count": count, "message": "ZIP de XMLs gerado com sucesso."}),
    )
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
fn write_u16(target: &mut File, value: u16) -> std::io::Result<()> {
    target.write_all(&value.to_le_bytes())
}
fn write_u32(target: &mut File, value: u32) -> std::io::Result<()> {
    target.write_all(&value.to_le_bytes())
}

fn write_stored_zip(path: &Path, entries: Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let mut file = File::create(path).map_err(|err| format!("Falha ao criar ZIP: {err}"))?;
    let mut central = Vec::new();
    let mut offset = 0u32;
    let entry_count = entries.len();
    for (name, content) in entries {
        let name_bytes = name.as_bytes();
        let crc = crc32(&content);
        let size = u32::try_from(content.len())
            .map_err(|_| "XML grande demais para exportação ZIP.".to_string())?;
        write_u32(&mut file, 0x0403_4b50).map_err(|err| err.to_string())?;
        write_u16(&mut file, 20).map_err(|err| err.to_string())?;
        write_u16(&mut file, 0x0800).map_err(|err| err.to_string())?;
        write_u16(&mut file, 0).map_err(|err| err.to_string())?;
        write_u16(&mut file, 0).map_err(|err| err.to_string())?;
        write_u16(&mut file, 0).map_err(|err| err.to_string())?;
        write_u32(&mut file, crc).map_err(|err| err.to_string())?;
        write_u32(&mut file, size).map_err(|err| err.to_string())?;
        write_u32(&mut file, size).map_err(|err| err.to_string())?;
        write_u16(
            &mut file,
            u16::try_from(name_bytes.len()).map_err(|_| "Nome de arquivo inválido.".to_string())?,
        )
        .map_err(|err| err.to_string())?;
        write_u16(&mut file, 0).map_err(|err| err.to_string())?;
        file.write_all(name_bytes)
            .and_then(|_| file.write_all(&content))
            .map_err(|err| format!("Falha ao escrever item no ZIP: {err}"))?;
        let mut header = Vec::new();
        header.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        header.extend_from_slice(&20u16.to_le_bytes());
        header.extend_from_slice(&20u16.to_le_bytes());
        header.extend_from_slice(&0x0800u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&crc.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        header.extend_from_slice(&offset.to_le_bytes());
        header.extend_from_slice(name_bytes);
        central.extend(header);
        offset = offset
            .checked_add(30 + name_bytes.len() as u32 + size)
            .ok_or_else(|| "ZIP excedeu o tamanho suportado.".to_string())?;
    }
    let central_offset = offset;
    file.write_all(&central)
        .map_err(|err| format!("Falha ao escrever diretório ZIP: {err}"))?;
    let central_size =
        u32::try_from(central.len()).map_err(|_| "Diretório ZIP grande demais.".to_string())?;
    let count = u16::try_from(entry_count)
        .map_err(|_| "Quantidade de XMLs excede o limite do formato ZIP.".to_string())?;
    write_u32(&mut file, 0x0605_4b50).map_err(|err| err.to_string())?;
    write_u16(&mut file, 0).map_err(|err| err.to_string())?;
    write_u16(&mut file, 0).map_err(|err| err.to_string())?;
    write_u16(&mut file, count).map_err(|err| err.to_string())?;
    write_u16(&mut file, count).map_err(|err| err.to_string())?;
    write_u32(&mut file, central_size).map_err(|err| err.to_string())?;
    write_u32(&mut file, central_offset).map_err(|err| err.to_string())?;
    write_u16(&mut file, 0).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(feature = "desktop")]
trait SmtpTransport: Read + Write {}

#[cfg(feature = "desktop")]
impl<T: Read + Write> SmtpTransport for T {}

#[cfg(feature = "desktop")]
enum SmtpConnection {
    Plain(TcpStream),
    Tls(Box<native_tls::TlsStream<TcpStream>>),
}

#[cfg(feature = "desktop")]
impl Read for SmtpConnection {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buffer),
            Self::Tls(stream) => stream.read(buffer),
        }
    }
}

#[cfg(feature = "desktop")]
impl Write for SmtpConnection {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buffer),
            Self::Tls(stream) => stream.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
        }
    }
}

#[cfg(feature = "desktop")]
fn smtp_read_response(stream: &mut dyn SmtpTransport) -> Result<String, String> {
    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream
            .read(&mut byte)
            .map_err(|err| format!("Falha ao ler resposta SMTP: {err}"))?;
        if read == 0 {
            return Err(
                "O servidor SMTP encerrou a conexão antes de concluir a resposta.".to_string(),
            );
        }
        response.push(byte[0]);
        if response.len() > 64 * 1024 {
            return Err("A resposta SMTP excedeu o limite de segurança.".to_string());
        }
        if byte[0] != b'\n' {
            continue;
        }
        let line = response
            .rsplit(|value| *value == b'\n')
            .next()
            .unwrap_or_default();
        if line.len() >= 4 && line[..3].iter().all(u8::is_ascii_digit) && line[3] == b' ' {
            return Ok(String::from_utf8_lossy(&response).into_owned());
        }
    }
}

#[cfg(feature = "desktop")]
fn smtp_code(response: &str) -> u16 {
    response
        .lines()
        .last()
        .and_then(|line| line.get(..3))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or_default()
}

#[cfg(feature = "desktop")]
fn smtp_expect(response: &str, accepted: &[u16]) -> Result<(), String> {
    let code = smtp_code(response);
    if accepted.contains(&code) {
        Ok(())
    } else {
        Err(format!(
            "Servidor SMTP retornou {code}: {}",
            response.trim()
        ))
    }
}

#[cfg(feature = "desktop")]
fn smtp_command(
    stream: &mut dyn SmtpTransport,
    command: &str,
    accepted: &[u16],
) -> Result<String, String> {
    stream
        .write_all(format!("{command}\r\n").as_bytes())
        .map_err(|err| format!("Falha ao enviar comando SMTP: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("Falha ao liberar comando SMTP: {err}"))?;
    let response = smtp_read_response(stream)?;
    smtp_expect(&response, accepted)?;
    Ok(response)
}

#[cfg(feature = "desktop")]
fn smtp_base64_lines(value: &[u8]) -> String {
    let encoded = BASE64_STANDARD.encode(value);
    encoded
        .as_bytes()
        .chunks(76)
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

#[cfg(feature = "desktop")]
fn smtp_header_value(value: &str, field: &str) -> Result<String, String> {
    if value.contains('\r') || value.contains('\n') {
        return Err(format!(
            "O campo SMTP {field} contém quebra de linha inválida."
        ));
    }
    Ok(value.trim().to_string())
}

#[cfg(feature = "desktop")]
#[allow(clippy::too_many_arguments)]
fn send_smtp_zip(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    sender: &str,
    recipient: &str,
    subject: &str,
    zip_name: &str,
    zip_bytes: &[u8],
) -> Result<(), String> {
    use native_tls::TlsConnector;

    let host = smtp_header_value(host, "servidor")?;
    let username = smtp_header_value(username, "conta")?;
    let sender = smtp_header_value(sender, "remetente")?;
    let recipient = smtp_header_value(recipient, "destinatário")?;
    let subject = smtp_header_value(subject, "assunto")?;
    let zip_name = smtp_header_value(zip_name, "anexo")?;
    if host.is_empty() || sender.is_empty() || recipient.is_empty() {
        return Err("Servidor, remetente e destinatário SMTP são obrigatórios.".to_string());
    }
    if !recipient.contains('@') || recipient.contains(' ') {
        return Err("O destinatário informado não parece ser um e-mail válido.".to_string());
    }

    let tcp = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("Falha ao conectar ao SMTP {host}:{port}: {err}"))?;
    tcp.set_read_timeout(Some(Duration::from_secs(20))).ok();
    tcp.set_write_timeout(Some(Duration::from_secs(20))).ok();
    let connector = TlsConnector::builder()
        .build()
        .map_err(|err| format!("Falha ao preparar TLS SMTP: {err}"))?;

    let mut stream = if port == 465 {
        let tls = connector
            .connect(&host, tcp)
            .map_err(|err| format!("Falha no TLS SMTP implícito: {err}"))?;
        SmtpConnection::Tls(Box::new(tls))
    } else {
        SmtpConnection::Plain(tcp)
    };

    let greeting = smtp_read_response(&mut stream)?;
    smtp_expect(&greeting, &[220])?;
    let ehlo = smtp_command(&mut stream, "EHLO manifestador-nfe", &[250])?;

    if port != 465 && ehlo.to_ascii_uppercase().contains("STARTTLS") {
        smtp_command(&mut stream, "STARTTLS", &[220])?;
        let plain = match stream {
            SmtpConnection::Plain(stream) => stream,
            SmtpConnection::Tls(_) => {
                return Err("Estado inválido ao preparar STARTTLS SMTP.".to_string())
            }
        };
        let tls = connector
            .connect(&host, plain)
            .map_err(|err| format!("Falha ao negociar STARTTLS SMTP: {err}"))?;
        stream = SmtpConnection::Tls(Box::new(tls));
        smtp_command(&mut stream, "EHLO manifestador-nfe", &[250])?;
    }

    if !username.is_empty() || !password.is_empty() {
        if username.is_empty() || password.is_empty() {
            return Err("A conta e a senha SMTP devem ser informadas juntas.".to_string());
        }
        smtp_command(&mut stream, "AUTH LOGIN", &[334])?;
        smtp_command(
            &mut stream,
            &BASE64_STANDARD.encode(username.as_bytes()),
            &[334],
        )?;
        smtp_command(
            &mut stream,
            &BASE64_STANDARD.encode(password.as_bytes()),
            &[235],
        )?;
    }

    smtp_command(&mut stream, &format!("MAIL FROM:<{sender}>"), &[250])?;
    smtp_command(&mut stream, &format!("RCPT TO:<{recipient}>"), &[250, 251])?;
    smtp_command(&mut stream, "DATA", &[354])?;

    let boundary = format!("=_ManifestadorNFe_{}", Utc::now().timestamp_millis());
    let message = format!(
        "From: {sender}\r\nTo: {recipient}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n--{boundary}\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\nArquivos XML de NF-e do Manifestador NF-e.\r\n\r\n--{boundary}\r\nContent-Type: application/zip; name=\"{zip_name}\"\r\nContent-Disposition: attachment; filename=\"{zip_name}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--{boundary}--\r\n.",
        smtp_base64_lines(zip_bytes)
    );
    stream
        .write_all(message.as_bytes())
        .map_err(|err| format!("Falha ao enviar corpo do e-mail: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("Falha ao liberar corpo do e-mail: {err}"))?;
    let response = smtp_read_response(&mut stream)?;
    smtp_expect(&response, &[250])?;
    let _ = smtp_command(&mut stream, "QUIT", &[221, 250]);
    Ok(())
}

#[cfg(not(feature = "desktop"))]
#[allow(clippy::too_many_arguments)]
fn send_smtp_zip(
    _host: &str,
    _port: u16,
    _username: &str,
    _password: &str,
    _sender: &str,
    _recipient: &str,
    _subject: &str,
    _zip_name: &str,
    _zip_bytes: &[u8],
) -> Result<(), String> {
    Err("O envio SMTP nativo está disponível somente no runtime desktop.".to_string())
}

pub fn send_accounting_email(
    conn: &Connection,
    data_dir: &Path,
    empresa_id: i64,
    data_inicial: &str,
    data_final: &str,
    recipient: &str,
) -> Result<Value, String> {
    let mut filters = Map::new();
    filters.insert("empresa_id".to_string(), Value::from(empresa_id));
    filters.insert(
        "data_inicial".to_string(),
        Value::String(data_inicial.to_string()),
    );
    filters.insert(
        "data_final".to_string(),
        Value::String(data_final.to_string()),
    );
    let notes = list_notes(conn, &filters)?;
    let mut entries = Vec::new();
    for note in notes {
        let id = note.get("id").and_then(Value::as_i64).unwrap_or_default();
        let chave = value_string(note.get("chave"));
        let (xml_content, xml_path): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT xml_content, xml_path FROM manifestador_notas WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|err| format!("Falha ao carregar XML para envio à contabilidade: {err}"))?;
        let xml = xml_content.or_else(|| xml_path.and_then(|path| fs::read_to_string(path).ok()));
        if let Some(xml) = xml {
            entries.push((
                format!(
                    "{}.xml",
                    if chave.is_empty() {
                        id.to_string()
                    } else {
                        chave
                    }
                ),
                xml.into_bytes(),
            ));
        }
    }
    if entries.is_empty() {
        return Err("Nenhum XML completo foi encontrado no período selecionado.".to_string());
    }
    let count = entries.len();
    let export_dir = data_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|err| format!("Falha ao criar diretório de exportação: {err}"))?;
    let zip_name = format!("notas-fiscais-{}.zip", Utc::now().format("%Y%m%d-%H%M%S"));
    let zip_path = export_dir.join(&zip_name);
    write_stored_zip(&zip_path, entries)?;
    let zip_bytes = fs::read(&zip_path)
        .map_err(|err| format!("Falha ao ler ZIP preparado para e-mail: {err}"))?;
    let (host, username, sender_name, sender_email, password_cipher, port): SmtpConfigRow = conn
        .query_row(
            "SELECT smtp_host, smtp_conta, smtp_remetente, smtp_email, smtp_senha_encrypted, smtp_porta FROM manifestador_empresa_operacao WHERE empresa_id = ?1",
            [empresa_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .map_err(|err| format!("Falha ao carregar configuração SMTP: {err}"))?;
    let host = host.unwrap_or_default();
    let username = username.unwrap_or_default();
    let sender = sender_email.unwrap_or_default();
    let password = password_cipher
        .map(|cipher| decrypt_text(&machine_key(), &cipher))
        .transpose()?
        .unwrap_or_default();
    let subject = format!(
        "NF-e Manifestador - período {} a {}{}",
        data_inicial,
        data_final,
        sender_name
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!(" - {value}"))
            .unwrap_or_default()
    );
    send_smtp_zip(
        &host,
        port.max(1) as u16,
        &username,
        &password,
        &sender,
        recipient,
        &subject,
        &zip_name,
        &zip_bytes,
    )?;
    Ok(json!({
        "ok": true,
        "path": zip_path.to_string_lossy(),
        "count": count,
        "recipient": recipient,
        "message": "ZIP de XMLs enviado à contabilidade com sucesso."
    }))
}

pub fn provider_endpoint(
    conn: &Connection,
    empresa_id: i64,
) -> Result<Option<(String, u64)>, String> {
    let row: Option<(Option<String>, i64)> = conn
        .query_row(
            "SELECT sefaz_endpoint, sefaz_timeout_seconds FROM manifestador_empresa_operacao WHERE empresa_id = ?1",
            [empresa_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| format!("Falha ao carregar adaptador externo: {err}"))?;
    Ok(row.and_then(|(endpoint, timeout)| {
        endpoint
            .filter(|value| !value.trim().is_empty())
            .map(|value| (value, timeout.max(5) as u64))
    }))
}

fn uf_code(uf: &str) -> i64 {
    match uf.trim().to_ascii_uppercase().as_str() {
        "RO" => 11,
        "AC" => 12,
        "AM" => 13,
        "RR" => 14,
        "PA" => 15,
        "AP" => 16,
        "TO" => 17,
        "MA" => 21,
        "PI" => 22,
        "CE" => 23,
        "RN" => 24,
        "PB" => 25,
        "PE" => 26,
        "AL" => 27,
        "SE" => 28,
        "BA" => 29,
        "MG" => 31,
        "ES" => 32,
        "RJ" => 33,
        "SP" => 35,
        "PR" => 41,
        "SC" => 42,
        "RS" => 43,
        "MS" => 50,
        "MT" => 51,
        "GO" => 52,
        "DF" => 53,
        _ => 0,
    }
}

fn environment_code(environment: &str) -> i64 {
    if matches!(
        environment.trim().to_ascii_lowercase().as_str(),
        "2" | "h" | "homologation" | "homologacao" | "homologação"
    ) {
        2
    } else {
        1
    }
}

fn default_distribution_url(environment: &str) -> &'static str {
    if environment_code(environment) == 2 {
        "https://hom.nfe.fazenda.gov.br/NFeDistribuicaoDFe/NFeDistribuicaoDFe.asmx"
    } else {
        "https://www1.nfe.fazenda.gov.br/NFeDistribuicaoDFe/NFeDistribuicaoDFe.asmx"
    }
}

fn xml_text_values(xml: &str) -> Result<HashMap<String, String>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut path: Vec<String> = Vec::new();
    let mut values = HashMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => path.push(local_name(event.name().as_ref())),
            Ok(Event::Text(event)) if !path.is_empty() => {
                values.insert(
                    path.join("/"),
                    event
                        .decode()
                        .map_err(|err| format!("Falha ao ler resposta XML: {err}"))?
                        .into_owned(),
                );
            }
            Ok(Event::CData(event)) if !path.is_empty() => {
                values.insert(
                    path.join("/"),
                    String::from_utf8_lossy(event.as_ref()).to_string(),
                );
            }
            Ok(Event::End(_)) => {
                path.pop();
            }
            Ok(Event::Empty(event)) => {
                if !path.is_empty() {
                    values.entry(path.join("/")).or_default();
                }
                let _ = event;
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("Resposta XML inválida: {err}")),
            _ => {}
        }
    }
    Ok(values)
}

fn parse_distribution_response(xml: &str) -> Result<Value, String> {
    let values = xml_text_values(xml)?;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut documents = Vec::new();
    let mut current_doc: Option<Map<String, Value>> = None;
    let mut path = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = local_name(event.name().as_ref());
                if tag == "docZip" {
                    let mut object = Map::new();
                    for attribute in event.attributes().with_checks(false) {
                        let attribute = attribute
                            .map_err(|err| format!("Falha ao ler atributo docZip: {err}"))?;
                        let key = local_name(attribute.key.as_ref()).to_ascii_lowercase();
                        let value = attribute
                            .normalized_value(XmlVersion::Implicit1_0)
                            .map_err(|err| format!("Falha ao ler atributo docZip: {err}"))?
                            .into_owned();
                        if key == "nsu" || key == "schema" {
                            object.insert(key, Value::String(value));
                        }
                    }
                    current_doc = Some(object);
                }
                path.push(tag);
            }
            Ok(Event::Text(event)) => {
                if let Some(object) = current_doc.as_mut() {
                    if path.last().map(String::as_str) == Some("docZip") {
                        let value = event
                            .decode()
                            .map_err(|err| format!("Falha ao ler conteúdo docZip: {err}"))?;
                        let current = object
                            .entry("docZip".to_string())
                            .or_insert_with(|| Value::String(String::new()));
                        if let Value::String(text) = current {
                            text.push_str(&value);
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = local_name(event.name().as_ref());
                if tag == "docZip" {
                    if let Some(object) = current_doc.take() {
                        documents.push(Value::Object(object));
                    }
                }
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(format!("Resposta de distribuição inválida: {err}")),
            _ => {}
        }
    }
    Ok(json!({
        "ok": matches!(path_value(&values, &["/cStat"]).as_str(), "" | "137" | "138"),
        "cStat": path_value(&values, &["/cStat"]),
        "xMotivo": path_value(&values, &["/xMotivo"]),
        "ultNSU": path_value(&values, &["/ultNSU"]),
        "maxNSU": path_value(&values, &["/maxNSU"]),
        "documentos": documents,
    }))
}

#[cfg(feature = "desktop")]
fn native_http_post(
    endpoint: &str,
    certificate_path: &str,
    certificate_password: &str,
    body: &str,
    timeout_seconds: u64,
) -> Result<String, String> {
    use native_tls::{Identity, TlsConnector};

    let url = Url::parse(endpoint).map_err(|err| format!("URL SEFAZ inválida: {err}"))?;
    if url.scheme() != "https" {
        return Err("A integração nativa SEFAZ exige endpoint HTTPS.".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Endpoint SEFAZ sem host.".to_string())?;
    let port = url.port_or_known_default().unwrap_or(443);
    let certificate = fs::read(certificate_path)
        .map_err(|err| format!("Falha ao ler certificado A1/PFX: {err}"))?;
    let identity = Identity::from_pkcs12(&certificate, certificate_password)
        .map_err(|err| format!("Falha ao abrir certificado A1/PFX: {err}"))?;
    let connector = TlsConnector::builder()
        .identity(identity)
        .build()
        .map_err(|err| format!("Falha ao preparar TLS com certificado A1: {err}"))?;
    let tcp = TcpStream::connect((host, port))
        .map_err(|err| format!("Falha ao conectar à SEFAZ {host}:{port}: {err}"))?;
    tcp.set_read_timeout(Some(Duration::from_secs(timeout_seconds)))
        .ok();
    tcp.set_write_timeout(Some(Duration::from_secs(timeout_seconds)))
        .ok();
    let mut stream = connector
        .connect(host, tcp)
        .map_err(|err| format!("Falha no TLS com certificado A1: {err}"))?;
    let mut target = url.path().to_string();
    if target.is_empty() {
        target.push('/');
    }
    if let Some(query) = url.query() {
        target.push('?');
        target.push_str(query);
    }
    let request = format!(
        "POST {target} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("Falha ao enviar requisição SOAP à SEFAZ: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("Falha ao liberar requisição SOAP à SEFAZ: {err}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("Falha ao ler resposta SOAP da SEFAZ: {err}"))?;
    let separator = b"\r\n\r\n";
    let body_start = response
        .windows(separator.len())
        .position(|window| window == separator)
        .map(|position| position + separator.len())
        .ok_or_else(|| "Resposta HTTP da SEFAZ sem cabeçalho completo.".to_string())?;
    let status_line = String::from_utf8_lossy(
        response
            .split(|byte| *byte == b'\n')
            .next()
            .unwrap_or_default(),
    );
    if !status_line.contains(" 200 ") && !status_line.contains(" 201 ") {
        return Err(format!("SEFAZ retornou HTTP {}", status_line.trim()));
    }
    String::from_utf8(response[body_start..].to_vec())
        .map_err(|err| format!("Resposta SOAP da SEFAZ não está em UTF-8: {err}"))
}

#[cfg(feature = "desktop")]
fn native_company_config(
    conn: &Connection,
    empresa_id: i64,
) -> Result<NativeCompanyConfig, String> {
    let row: NativeCompanyConfigRow = conn
        .query_row(
            "SELECT e.documento, c.certificado_path, c.certificado_password_encrypted, c.uf, c.ambiente, c.nsu, g.sefaz_service_url, g.sefaz_timeout_seconds FROM empresas e INNER JOIN manifestador_empresa_config c ON c.empresa_id = e.id INNER JOIN manifestador_empresa_operacao g ON g.empresa_id = e.id WHERE e.id = ?1 AND e.ativo = 1",
            [empresa_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
        )
        .map_err(|err| format!("Falha ao carregar certificado da empresa: {err}"))?;
    let password = row
        .2
        .map(|value| decrypt_text(&format!("{}:{empresa_id}", machine_key()), &value))
        .transpose()?
        .unwrap_or_default();
    Ok((
        normalize_document(row.0.as_deref().unwrap_or_default()),
        row.1.unwrap_or_default(),
        password,
        row.3.unwrap_or_default(),
        row.4.unwrap_or_else(|| "production".to_string()),
        row.5.unwrap_or_else(|| "0".to_string()),
        row.6.unwrap_or_default(),
        row.7.max(5) as u64,
    ))
}

#[cfg(feature = "desktop")]
fn native_distribution_request(
    conn: &Connection,
    empresa_id: i64,
    inner: &str,
) -> Result<Value, String> {
    let (
        _cnpj,
        certificate_path,
        certificate_password,
        uf,
        environment,
        _nsu,
        configured_url,
        timeout,
    ) = native_company_config(conn, empresa_id)?;
    if certificate_path.trim().is_empty() {
        return Err(
            "Configure o caminho do certificado A1/PFX da empresa antes de consultar a SEFAZ."
                .to_string(),
        );
    }
    if uf_code(&uf) == 0 {
        return Err("Configure a UF da empresa antes de consultar a SEFAZ.".to_string());
    }
    let endpoint = if configured_url.trim().is_empty() {
        default_distribution_url(&environment).to_string()
    } else {
        configured_url
    };
    let soap = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><soap12:Envelope xmlns:soap12="http://www.w3.org/2003/05/soap-envelope"><soap12:Body><nfeDistDFeInteresse xmlns="http://www.portalfiscal.inf.br/nfe/wsdl/NFeDistribuicaoDFe"><nfeDadosMsg>{inner}</nfeDadosMsg></nfeDistDFeInteresse></soap12:Body></soap12:Envelope>"#
    );
    let response = native_http_post(
        &endpoint,
        &certificate_path,
        &certificate_password,
        &soap,
        timeout,
    )?;
    parse_distribution_response(&response)
}

#[cfg(feature = "desktop")]
pub fn native_obter_notas(conn: &Connection, empresa_id: i64) -> Result<Value, String> {
    let (cnpj, _certificate_path, _certificate_password, uf, environment, nsu, _url, _timeout) =
        native_company_config(conn, empresa_id)?;
    let ult_nsu = nsu.trim().parse::<u64>().unwrap_or_default();
    let inner = format!(
        r#"<distDFeInt xmlns="http://www.portalfiscal.inf.br/nfe" versao="1.01"><tpAmb>{}</tpAmb><cUFAutor>{:02}</cUFAutor><CNPJ>{cnpj}</CNPJ><distNSU><ultNSU>{ult_nsu:015}</ultNSU></distNSU></distDFeInt>"#,
        environment_code(&environment),
        uf_code(&uf),
    );
    let result = native_distribution_request(conn, empresa_id, &inner)?;
    if let Some(ult_nsu) = result
        .get("ultNSU")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        conn.execute(
            "UPDATE manifestador_empresa_config SET nsu = ?1, updated_at = ?2 WHERE empresa_id = ?3",
            params![ult_nsu, Utc::now().to_rfc3339(), empresa_id],
        )
        .map_err(|err| format!("Falha ao atualizar NSU da empresa: {err}"))?;
    }
    Ok(result)
}

#[cfg(not(feature = "desktop"))]
pub fn native_obter_notas(_conn: &Connection, _empresa_id: i64) -> Result<Value, String> {
    Err("A consulta nativa à SEFAZ está disponível somente no aplicativo desktop.".to_string())
}

#[cfg(feature = "desktop")]
pub fn native_download_note(
    conn: &Connection,
    empresa_id: i64,
    nota_id: i64,
    data_dir: &Path,
) -> Result<Value, String> {
    let (chave,): (String,) = conn
        .query_row(
            "SELECT chave FROM manifestador_notas WHERE id = ?1 AND empresa_id = ?2",
            params![nota_id, empresa_id],
            |row| Ok((row.get(0)?,)),
        )
        .map_err(|err| format!("Falha ao localizar NF-e para download SEFAZ: {err}"))?;
    let (_cnpj, _certificate_path, _certificate_password, _uf, environment, _nsu, _url, _timeout) =
        native_company_config(conn, empresa_id)?;
    let inner = format!(
        r#"<consChNFe xmlns="http://www.portalfiscal.inf.br/nfe" versao="1.01"><tpAmb>{}</tpAmb><xServ>CONSULTAR NFE</xServ><chNFe>{chave}</chNFe></consChNFe>"#,
        environment_code(&environment),
    );
    let result = native_distribution_request(conn, empresa_id, &inner)?;
    let documents = result
        .get("documentos")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for document in documents {
        let schema = document
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let encoded = document
            .get("docZip")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if schema.to_ascii_lowercase().contains("procnfe") && !encoded.is_empty() {
            let xml = decode_doczip(encoded)?;
            return download_note(conn, data_dir, empresa_id, nota_id, Some(&xml));
        }
    }
    Err(format!(
        "A SEFAZ não retornou o XML processado da NF-e. {}",
        result
            .get("xMotivo")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Consulte novamente após a autorização do documento.")
    ))
}

#[cfg(not(feature = "desktop"))]
pub fn native_download_note(
    _conn: &Connection,
    _empresa_id: i64,
    _nota_id: i64,
    _data_dir: &Path,
) -> Result<Value, String> {
    Err("O download nativo da SEFAZ está disponível somente no aplicativo desktop.".to_string())
}

fn distribution_summary_from_xml(xml: &str, nsu: i64) -> Result<Map<String, Value>, String> {
    let values = xml_text_values(xml)?;
    let has_summary_root = values.keys().any(|key| {
        key.split('/')
            .any(|part| matches!(part, "resNFe" | "resEvento" | "resCTe" | "resMDFe"))
    });
    if !has_summary_root {
        return Err("O XML recebido não é um resumo de distribuição NF-e.".to_string());
    }
    let chave = path_value(&values, &["/chNFe", "/chDFe"]);
    if chave.trim().is_empty() {
        return Err("O resumo XML recebido não contém a chave de acesso.".to_string());
    }
    json!({
        "chave": chave,
        "numero": path_value(&values, &["/nNF"]),
        "serie": path_value(&values, &["/serie"]),
        "cnpj": path_value(&values, &["/CNPJ", "/CNPJCPF", "/CPF"]),
        "nome": path_value(&values, &["/xNome"]),
        "ie": path_value(&values, &["/IE"]),
        "emissao": path_value(&values, &["/dhEmi", "/dEmi"]),
        "valor": path_value(&values, &["/vNF"]),
        "tipo": path_value(&values, &["/tpNF"]),
        "ambiente": path_value(&values, &["/tpAmb"]),
        "situacao": path_value(&values, &["/cSitNFe", "/cSitDFe"]),
        "protocolo": path_value(&values, &["/nProt"]),
        "data_recebimento": path_value(&values, &["/dhRecbto"]),
        "nsu": nsu,
    })
    .as_object()
    .cloned()
    .ok_or_else(|| "Falha ao montar resumo XML de distribuição.".to_string())
}

fn xml_is_distribution_summary(xml: &str) -> Result<bool, String> {
    let values = xml_text_values(xml)?;
    Ok(values.keys().any(|key| {
        key.split('/')
            .any(|part| matches!(part, "resNFe" | "resEvento" | "resCTe" | "resMDFe"))
    }))
}

/// Importa um documento retornado por um adaptador ou pela consulta nativa.
/// A DistribuicaoDFe pode entregar XML completo, resumo `resNFe`/`resEvento`
/// compactado em `docZip` ou um objeto JSON já normalizado pelo adaptador.
pub fn import_provider_document(
    conn: &mut Connection,
    empresa_id: i64,
    document: &Value,
) -> Result<bool, String> {
    if let Some(text) = document.as_str().filter(|value| !value.trim().is_empty()) {
        if xml_is_distribution_summary(text)? {
            let object = distribution_summary_from_xml(text, 0)?;
            import_summary(conn, empresa_id, &object, 0, None)?;
        } else {
            import_xml(conn, empresa_id, text, 0, None)?;
        }
        return Ok(true);
    }

    let Some(object) = document.as_object() else {
        return Ok(false);
    };
    let nsu = object_i64(object, &["nsu", "NSU"]);
    let mut xml = value_string(object.get("xml"));
    if xml.is_empty() {
        xml = value_string(object.get("docZip"));
        if !xml.is_empty() {
            xml = decode_doczip(&xml)?;
        }
    }
    if xml.is_empty() {
        import_summary(conn, empresa_id, object, nsu, None)?;
    } else if xml_is_distribution_summary(&xml)? {
        let summary = distribution_summary_from_xml(&xml, nsu)?;
        import_summary(conn, empresa_id, &summary, nsu, None)?;
    } else {
        import_xml(conn, empresa_id, &xml, nsu, None)?;
    }
    Ok(true)
}

pub fn call_provider(
    conn: &Connection,
    action: &str,
    payload: &Value,
) -> Result<Option<Value>, String> {
    let empresa_id = payload
        .get("empresa_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or("Selecione uma empresa para acessar o provedor fiscal.")?;
    let Some((endpoint, timeout)) = provider_endpoint(conn, empresa_id)? else {
        return Ok(None);
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()
        .map_err(|err| format!("Falha ao preparar adaptador externo: {err}"))?;
    let response = client
        .post(endpoint)
        .header("X-Manifestador-Action", action)
        .json(payload)
        .send()
        .map_err(|err| format!("Falha ao chamar adaptador externo do Manifestador: {err}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|err| format!("Falha ao ler resposta do adaptador externo: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "Adaptador externo retornou HTTP {}: {}",
            status.as_u16(),
            body.chars().take(300).collect::<String>()
        ));
    }
    let value = serde_json::from_str::<Value>(&body)
        .map_err(|err| format!("Adaptador externo retornou JSON inválido: {err}"))?;
    Ok(Some(value))
}

/// Executa uma rodada do agendamento por empresa. A rodada é deliberadamente
/// idempotente: o NSU e a chave são preservados no banco, e cada documento
/// recebido pela integração nativa ou pelo adaptador é atualizado em vez de duplicado.
fn claim_company_cycle(conn: &Connection, empresa_id: i64, now: i64) -> Result<bool, String> {
    conn.execute("UPDATE manifestador_empresa_operacao SET last_check_at = ?2 WHERE empresa_id = ?1 AND verificar_automatico <> 0 AND (last_check_at IS NULL OR last_check_at <= ?2 - MAX(1, timer_minutos) * 60)", params![empresa_id, now]).map(|count| count > 0).map_err(|e| e.to_string())
}

pub fn automatic_cycle(conn: &mut Connection, data_dir: &Path) -> Result<Value, String> {
    let company_ids = {
        let mut stmt = conn
            .prepare("SELECT id FROM empresas WHERE ativo = 1 ORDER BY id")
            .map_err(|err| format!("Falha ao preparar empresas do timer: {err}"))?;
        let ids = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|err| format!("Falha ao consultar empresas do timer: {err}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Falha ao mapear empresas do timer: {err}"))?;
        ids
    };
    let mut imported = 0i64;
    let mut companies = 0i64;
    for company_id in company_ids {
        ensure_operation_config(conn, company_id)?;
        // Claim only due companies. Persist attempts too, so failures cannot hammer SEFAZ.
        if !claim_company_cycle(conn, company_id, Utc::now().timestamp())? {
            continue;
        }
        let download_automatically: i64 = conn.query_row("SELECT download_automatico FROM manifestador_empresa_operacao WHERE empresa_id = ?1", [company_id], |r| r.get(0)).map_err(|e| e.to_string())?;
        let external_provider_configured = provider_endpoint(conn, company_id)?.is_some();
        let request = json!({"empresa_id": company_id, "automatico": true});
        let response = if let Some(response) = call_provider(conn, "obter_notas", &request)? {
            response
        } else {
            match native_obter_notas(conn, company_id) {
                Ok(response) => response,
                Err(_) => continue,
            }
        };
        let documents = response
            .get("documentos")
            .or_else(|| response.get("notas"))
            .or_else(|| response.get("xmls"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut company_imported = 0i64;
        for document in documents {
            if let Some(xml) = document.as_str().filter(|value| !value.trim().is_empty()) {
                if import_provider_document(conn, company_id, &Value::String(xml.to_string()))? {
                    company_imported += 1;
                }
                continue;
            }
            let Some(object) = document.as_object() else {
                continue;
            };
            if !import_provider_document(conn, company_id, &document)? {
                continue;
            }
            if download_automatically != 0 {
                let chave = object_string(object, &["chave", "chDFe", "chNFe"]);
                let note_id = if !chave.is_empty() {
                    conn.query_row(
                        "SELECT id FROM manifestador_notas WHERE empresa_id = ?1 AND chave = ?2 ORDER BY id DESC LIMIT 1",
                        params![company_id, chave.as_str()],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .map_err(|err| format!("Falha ao localizar NF-e recebida pelo timer: {err}"))?
                    .unwrap_or_default()
                } else {
                    0
                };
                if note_id > 0 && !chave.is_empty() {
                    if external_provider_configured {
                        if let Some(download_response) = call_provider(
                            conn,
                            "download",
                            &json!({"empresa_id": company_id, "nota_id": note_id, "chave": chave, "nsu": object_i64(object, &["nsu", "NSU"])}),
                        )? {
                            let mut full_xml = download_response
                                .get("xml")
                                .map(|value| value_string(Some(value)))
                                .unwrap_or_default();
                            if full_xml.is_empty() {
                                full_xml = download_response
                                    .get("docZip")
                                    .map(|value| value_string(Some(value)))
                                    .unwrap_or_default();
                                if !full_xml.is_empty() {
                                    full_xml = decode_doczip(&full_xml)?;
                                }
                            }
                            if !full_xml.is_empty() {
                                download_note(
                                    conn,
                                    data_dir,
                                    company_id,
                                    note_id,
                                    Some(&full_xml),
                                )?;
                            }
                        }
                    } else {
                        let _ = native_download_note(conn, company_id, note_id, data_dir);
                    }
                }
            }
            company_imported += 1;
        }
        conn.execute(
            "UPDATE manifestador_empresa_config SET data_manifestacao = ?1, updated_at = ?1 WHERE empresa_id = ?2",
            params![Utc::now().to_rfc3339(), company_id],
        )
        .map_err(|err| format!("Falha ao atualizar último ciclo da empresa: {err}"))?;
        imported += company_imported;
        companies += 1;
    }
    Ok(json!({"ok": true, "companies": companies, "imported": imported}))
}

#[allow(dead_code)]
pub fn read_certificate_password(
    conn: &Connection,
    empresa_id: i64,
) -> Result<Option<String>, String> {
    let cipher: Option<String> = conn.query_row("SELECT certificado_password_encrypted FROM manifestador_empresa_config WHERE empresa_id = ?1", [empresa_id], |row| row.get(0)).optional().map_err(|err| format!("Falha ao ler senha do certificado: {err}"))?.flatten();
    cipher
        .map(|value| decrypt_text(&format!("{}:{empresa_id}", machine_key()), &value))
        .transpose()
}

#[allow(dead_code)]
pub fn read_smtp_password(conn: &Connection, empresa_id: i64) -> Result<Option<String>, String> {
    let cipher: Option<String> = conn
        .query_row(
            "SELECT smtp_senha_encrypted FROM manifestador_empresa_operacao WHERE empresa_id = ?1",
            [empresa_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Falha ao ler senha SMTP: {err}"))?
        .flatten();
    cipher
        .map(|value| decrypt_text(&machine_key(), &value))
        .transpose()
}

#[allow(dead_code)]
pub fn smtp_probe(host: &str, port: u16) -> Result<Value, String> {
    let mut stream = TcpStream::connect((host, port))
        .map_err(|err| format!("Falha ao conectar ao SMTP: {err}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let mut buffer = [0u8; 512];
    let read = stream.read(&mut buffer).unwrap_or(0);
    Ok(json!({"connected": true, "banner": String::from_utf8_lossy(&buffer[..read]).trim()}))
}

#[cfg(test)]
mod tests {
    use super::{distribution_summary_from_xml, parse_nfe_xml};

    fn company_fixture() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE empresas (id INTEGER PRIMARY KEY, ativo INTEGER); INSERT INTO empresas VALUES (1,1),(2,1);").unwrap();
        super::migrate(&conn).unwrap();
        super::ensure_company_configs(&conn).unwrap();
        conn
    }

    #[test]
    fn migrates_shared_settings_once_and_isolates_companies() {
        let conn = company_fixture();
        conn.execute_batch("DROP TABLE manifestador_empresa_operacao; UPDATE manifestador_config SET smtp_host='original', timer_minutos=120;").unwrap();
        let encrypted = crate::security::encrypt_text(&super::machine_key(), "old-secret");
        conn.execute(
            "UPDATE manifestador_config SET smtp_senha_encrypted=?1",
            [encrypted],
        )
        .unwrap();
        super::migrate_company_operation_config(&conn).unwrap();
        assert_eq!(
            super::get_config(&conn, 1).unwrap()["smtp_host"],
            "original"
        );
        let mut payload = super::get_config(&conn, 1)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        payload.insert("smtp_host".into(), serde_json::json!("company-one"));
        payload.insert("smtp_password".into(), serde_json::json!(""));
        payload.insert(
            "sefaz_endpoint".into(),
            serde_json::json!("https://one.invalid"),
        );
        super::save_config(&conn, 1, &payload).unwrap();
        super::migrate(&conn).unwrap();
        assert_eq!(
            super::get_config(&conn, 1).unwrap()["smtp_host"],
            "company-one"
        );
        assert_eq!(
            super::get_config(&conn, 2).unwrap()["smtp_host"],
            "original"
        );
        assert_eq!(
            super::read_smtp_password(&conn, 1).unwrap().as_deref(),
            Some("old-secret")
        );
        assert!(super::get_config(&conn, 1)
            .unwrap()
            .get("smtp_senha_encrypted")
            .is_none());
        assert_eq!(
            super::provider_endpoint(&conn, 1).unwrap().unwrap().0,
            "https://one.invalid"
        );
        assert!(super::provider_endpoint(&conn, 2).unwrap().is_none());
        conn.execute("INSERT INTO empresas VALUES(3,1)", [])
            .unwrap();
        assert!(super::get_config(&conn, 3).unwrap()["smtp_host"].is_null());
        assert!(super::get_company_config(&conn, 3).is_ok());
    }

    #[test]
    fn combined_company_save_is_atomic_and_preserves_secrets() {
        let conn = company_fixture();
        let original = serde_json::json!({"certificado_password":"certificate-secret", "uf":"SP", "nsu":"42", "operation_config":{"smtp_password":"mail-secret", "smtp_host":"one"}});
        super::save_company_config(&conn, 1, original.as_object().unwrap()).unwrap();
        let mut payload = super::get_company_config(&conn, 1)
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        payload.insert(
            "operation_config".into(),
            serde_json::json!({"smtp_host":"changed", "smtp_password":""}),
        );
        conn.execute_batch("CREATE TRIGGER fail_company BEFORE UPDATE ON manifestador_empresa_config BEGIN SELECT RAISE(ABORT, 'test rollback'); END;").unwrap();
        assert!(super::save_company_config(&conn, 1, &payload).is_err());
        assert_eq!(super::get_config(&conn, 1).unwrap()["smtp_host"], "one");
        conn.execute_batch("DROP TRIGGER fail_company").unwrap();
        super::save_company_config(&conn, 1, &payload).unwrap();
        assert_eq!(super::get_company_config(&conn, 1).unwrap()["nsu"], "42");
        assert_eq!(
            super::read_smtp_password(&conn, 1).unwrap().as_deref(),
            Some("mail-secret")
        );
        assert!(
            super::get_company_config(&conn, 1).unwrap()["certificado_password_configured"]
                .as_bool()
                .unwrap()
        );
        assert!(super::get_config(&conn, 2).unwrap()["smtp_host"].is_null());
        payload.remove("nsu");
        payload.remove("data_manifestacao");
        conn.execute("UPDATE manifestador_empresa_config SET nsu='99', data_manifestacao='latest' WHERE empresa_id=1", []).unwrap();
        super::save_company_config(&conn, 1, &payload).unwrap();
        let saved = super::get_company_config(&conn, 1).unwrap();
        assert_eq!(saved["nsu"], "99");
        assert_eq!(saved["data_manifestacao"], "latest");
    }

    #[test]
    fn automatic_schedule_obeys_each_company_interval_and_enabled_flag() {
        let conn = company_fixture();
        conn.execute_batch("UPDATE manifestador_empresa_operacao SET timer_minutos=5 WHERE empresa_id=1; UPDATE manifestador_empresa_operacao SET verificar_automatico=0 WHERE empresa_id=2;").unwrap();
        assert!(super::claim_company_cycle(&conn, 1, 1000).unwrap());
        assert!(!super::claim_company_cycle(&conn, 1, 1299).unwrap());
        assert!(super::claim_company_cycle(&conn, 1, 1300).unwrap());
        assert!(!super::claim_company_cycle(&conn, 2, 1300).unwrap());
    }

    #[test]
    fn importa_xml_minimo_com_chave_itens_e_totais() {
        let xml = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/manifestador.xml"
        ));
        let parsed = parse_nfe_xml(xml).expect("XML de fixture deveria ser aceito");
        assert_eq!(parsed.chave, "35261111111111111111550010000000011000000010");
        assert_eq!(parsed.numero, 1);
        assert_eq!(parsed.serie, 1);
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].cprod, "001");
        assert!((parsed.valor - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn converte_resumo_resnfe_da_distribuicao() {
        let xml = r#"<resNFe xmlns="http://www.portalfiscal.inf.br/nfe" versao="1.01"><chNFe>35261111111111111111550010000000011000000010</chNFe><CNPJ>11111111111111</CNPJ><xNome>Empresa de teste</xNome><dhEmi>2026-11-04T10:00:00-03:00</dhEmi><vNF>20.00</vNF><tpNF>1</tpNF><cSitNFe>1</cSitNFe><tpAmb>2</tpAmb></resNFe>"#;
        let summary =
            distribution_summary_from_xml(xml, 42).expect("resumo deveria ser convertido");
        assert_eq!(
            summary.get("chave").and_then(|value| value.as_str()),
            Some("35261111111111111111550010000000011000000010")
        );
        assert_eq!(
            summary.get("nsu").and_then(|value| value.as_i64()),
            Some(42)
        );
        assert_eq!(
            summary.get("tipo").and_then(|value| value.as_str()),
            Some("1")
        );
    }
}
