//! Motor fiscal embarcado: schemas NF-e/NFC-e, libxml2 estática e DANFE em Rust.
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{path::Path, sync::Mutex};
use base64::{engine::general_purpose::STANDARD, Engine as _};
static ENGINE_LOCK: Mutex<()> = Mutex::new(());

const SCHEMAS: &[(&str, &[u8])] = &[
    (
        "nfe_v4.00.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/nfe_v4.00.xsd"),
    ),
    (
        "leiauteNFe_v4.00.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/leiauteNFe_v4.00.xsd"),
    ),
    (
        "xmldsig-core-schema_v1.01.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/xmldsig-core-schema_v1.01.xsd"),
    ),
    (
        "tiposBasico_v4.00.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/tiposBasico_v4.00.xsd"),
    ),
    (
        "DFeTiposBasicos_v1.00.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/DFeTiposBasicos_v1.00.xsd"),
    ),
    (
        "procNFe_v4.00.xsd",
        include_bytes!("../resources/fiscal/schemas/nfe/procNFe_v4.00.xsd"),
    ),
];

fn bundled_schemas(data_dir: &Path) -> Result<std::path::PathBuf, String> {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let mut hash = Sha256::new();
    for (name, bytes) in SCHEMAS {
        hash.update(name);
        hash.update(bytes);
    }
    let directory = data_dir
        .join("fiscal-schemas")
        .join(format!("{:x}", hash.finalize()));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    for (name, bytes) in SCHEMAS {
        let path = directory.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => file.write_all(bytes).map_err(|e| e.to_string())?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::read(&path).map_err(|e| e.to_string())? != *bytes {
                    return Err(format!("Schema embarcado alterado: {name}. Restaure o pacote de schemas da aplicação."));
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(directory)
}

#[derive(Debug, PartialEq)]
enum DocumentKind {
    Nfe,
    Nfce,
}
impl DocumentKind {
    fn model(&self) -> u16 {
        match self {
            Self::Nfe => 55,
            Self::Nfce => 65,
        }
    }
}

pub fn status() -> Value {
    json!({"supported":true,"configured":true,"engine":"Rust + libxml2 2.15.4 embarcada",
        "models":[55,65],"message":"DANFE e validação XSD de NF-e e NFC-e disponíveis localmente, sem instalação adicional."})
}

fn identify(xml: &str) -> Result<(DocumentKind, bool), String> {
    if xml.len() > 8 * 1024 * 1024 || xml.contains('\0') {
        return Err("XML inválido ou maior que 8 MiB.".into());
    }
    let mut reader = quick_xml::Reader::from_str(xml);
    let (mut nfe, mut info, mut depth, mut processed) = (0, 0, 0, false);
    loop {
        match reader.read_event().map_err(|_| "XML malformado.")? {
            quick_xml::events::Event::DocType(_) => {
                return Err("DTD e entidades externas não são permitidas.".into())
            }
            quick_xml::events::Event::Start(e) => {
                depth += 1;
                if depth > 128 {
                    return Err("XML excede a profundidade permitida.".into());
                }
                match e.local_name().as_ref() {
                    b"nfeProc" => processed = true,
                    b"NFe" => nfe += 1,
                    b"infNFe" => {
                        info += 1;
                        let version = e
                            .try_get_attribute("versao")
                            .map_err(|_| "Versão XML inválida.")?;
                        if version.as_ref().map(|v| v.value.as_ref()) != Some(b"4.00".as_slice()) {
                            return Err("Versão de documento ainda não suportada. NF-e e NFC-e devem usar o leiaute 4.00.".into());
                        }
                    }
                    _ => {}
                }
            }
            quick_xml::events::Event::End(_) => depth -= 1,
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    if nfe != 1 || info != 1 {
        return Err("Informe um XML completo de NF-e ou NFC-e. Resumos e outros documentos fiscais não possuem renderizador nesta versão.".into());
    }
    let parsed = crate::manifestador::parse_nfe_xml(xml)?;
    if parsed.chave.len() != 44 || !parsed.chave.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Chave de acesso inválida.".into());
    }
    let kind = match &parsed.chave[20..22] {
        "55" => DocumentKind::Nfe,
        "65" => DocumentKind::Nfce,
        _ => return Err("Modelo fiscal ainda não suportado para PDF/XSD.".into()),
    };
    Ok((kind, processed))
}

fn run(xml: &str, data_dir: &Path, pdf: bool) -> Result<Value, String> {
    let (kind, processed) = identify(xml)?;
    let _lock = ENGINE_LOCK
        .lock()
        .map_err(|_| "Motor fiscal indisponível após falha anterior.")?;
    let schemas = bundled_schemas(data_dir)?;
    let schema = if processed {
        "procNFe_v4.00.xsd"
    } else {
        "nfe_v4.00.xsd"
    };
    manifestador_fiscal_xml::validate(xml, &schemas.join(schema))?;
    if !pdf {
        return Ok(
            json!({"valid":true,"model":kind.model(),"engine":"libxml2 2.15.4 estática","schema":schema,"message":"XML aprovado nos schemas embarcados. Assinatura criptográfica e situação atual na SEFAZ não foram verificadas."}),
        );
    }
    let bytes = danfe::render_default(xml)
        .map_err(|e| format!("Falha ao gerar documento auxiliar: {e}"))?;
    if !bytes.starts_with(b"%PDF-") || bytes.len() > 32 * 1024 * 1024 {
        return Err("PDF inválido ou maior que 32 MiB.".into());
    }
    Ok(
        json!({"engine":"danfe 0.1.0 / Rust","model":kind.model(),"mime_type":"application/pdf","content_base64":STANDARD.encode(bytes)}),
    )
}

pub fn process(
    conn: &Connection,
    data_dir: &Path,
    company: i64,
    note: i64,
    operation: &str,
) -> Result<Value, String> {
    if !matches!(operation, "validate" | "danfe") {
        return Err("Operação fiscal desconhecida.".into());
    }
    let record: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT xml_content,xml_path FROM manifestador_notas WHERE empresa_id=?1 AND id=?2",
            params![company, note],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (content, path) = record.ok_or("Documento não pertence à empresa ativa.")?;
    let xml = match content.filter(|s| !s.trim().is_empty()) {
        Some(xml) => xml,
        None => {
            use std::io::Read;
            let file =
                std::fs::File::open(path.ok_or("Baixe o XML completo antes desta operação.")?)
                    .map_err(|_| "Não foi possível ler o XML.")?;
            let mut xml = String::new();
            file.take(8 * 1024 * 1024 + 1)
                .read_to_string(&mut xml)
                .map_err(|_| "XML não é UTF-8 válido.")?;
            xml
        }
    };
    let mut result = run(&xml, data_dir, operation == "danfe")?;
    if operation == "danfe" {
        let bytes = STANDARD
            .decode(result["content_base64"].as_str().ok_or("PDF ausente.")?)
            .map_err(|_| "PDF inválido.")?;
        let dir = data_dir.join("exports").join(format!("empresa-{company}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let filename = format!("danfe-{note}-{}.pdf", uuid::Uuid::new_v4());
        let path = dir.join(&filename);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        result["file_name"] = Value::String(filename);
        result["path"] = Value::String(path.to_string_lossy().into_owned());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    const NFE: &str = include_str!("../../tests/fiscal/nfe-homologacao.xml");
    const NFCE: &str = include_str!("../../tests/fiscal/nfce-homologacao.xml");
    #[test]
    fn embedded_real_xml_validation_and_pdf_both_models() {
        let dir = std::env::temp_dir().join(format!("fiscal-real-{}", uuid::Uuid::new_v4()));
        for (xml, model) in [(NFE, 55), (NFCE, 65)] {
            let valid = run(xml, &dir, false).unwrap();
            assert_eq!(valid["model"], model);
            let pdf = run(xml, &dir, true).unwrap();
            assert!(STANDARD
                .decode(pdf["content_base64"].as_str().unwrap())
                .unwrap()
                .starts_with(b"%PDF-"));
            let invalid = xml
                .replace("<mod>55</mod>", "<mod>99</mod>")
                .replace("<mod>65</mod>", "<mod>99</mod>");
            assert!(run(&invalid, &dir, false).is_err());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejects_unsafe_inputs_and_foreign_company() {
        assert!(
            identify("<!DOCTYPE NFe [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><NFe/>").is_err()
        );
        assert!(identify("<resNFe/>").is_err());
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE manifestador_notas(id INTEGER,empresa_id INTEGER,xml_content TEXT,xml_path TEXT);INSERT INTO manifestador_notas VALUES(1,1,'<NFe/>',NULL);").unwrap();
        assert!(process(&conn, Path::new("."), 2, 1, "danfe")
            .unwrap_err()
            .contains("não pertence"));
    }
    #[test]
    fn bundled_schema_tampering_is_rejected() {
        let root = std::env::temp_dir().join(format!("fiscal-schema-{}", uuid::Uuid::new_v4()));
        let path = bundled_schemas(&root).unwrap();
        std::fs::write(path.join("nfe_v4.00.xsd"), "changed").unwrap();
        assert!(bundled_schemas(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
