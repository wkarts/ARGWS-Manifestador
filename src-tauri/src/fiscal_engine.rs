//! Adaptador ACBrLibNFe MT/cdecl. A biblioteca é instalada pelo administrador;
//! não há substituto fictício para XSD ou DANFE quando ela estiver ausente.
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;

const SUPPORTED: bool = cfg!(all(
    any(target_os = "windows", target_os = "linux"),
    any(target_arch = "x86", target_arch = "x86_64")
));

pub fn status() -> Value {
    let configured = std::env::var_os("MANIFESTADOR_ACBR_NFE_MT_CDECL_LIBRARY").is_some()
        && std::env::var_os("MANIFESTADOR_ACBR_SCHEMAS").is_some();
    json!({"supported":SUPPORTED,"configured":configured,"engine":"ACBrLibNFe MT/cdecl",
        "message": if !SUPPORTED {"DANFE e XSD via ACBrLib exigem servidor Windows/Linux x86 ou x64. Use a conexão REST com um servidor configurado."}
        else if !configured {"ACBrLib e schemas não configurados neste servidor. DANFE PDF e validação XSD indisponíveis."}
        else {"ACBrLib configurada. A disponibilidade e os schemas serão verificados ao executar a operação."}})
}

fn check_xml(xml: &str) -> Result<(), String> {
    if xml.len() > 8 * 1024 * 1024 || xml.contains('\0') {
        return Err("XML inválido ou maior que 8 MiB.".into());
    }
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut nfe_count = 0;
    let mut info_count = 0;
    loop {
        match reader.read_event().map_err(|_| "XML malformado.")? {
            quick_xml::events::Event::DocType(_) => {
                return Err("DTD e entidades externas não são permitidas.".into())
            }
            quick_xml::events::Event::Start(e) if e.local_name().as_ref() == b"NFe" => {
                nfe_count += 1
            }
            quick_xml::events::Event::Start(e) if e.local_name().as_ref() == b"infNFe" => {
                info_count += 1;
                let version = e
                    .try_get_attribute("versao")
                    .map_err(|_| "Versão XML inválida.")?;
                if version.as_ref().map(|a| a.value.as_ref()) != Some(b"4.00".as_slice()) {
                    return Err("Este recurso aceita NF-e versão 4.00.".into());
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    if nfe_count != 1 || info_count != 1 {
        return Err("Baixe o XML completo de uma NF-e. Resumos de distribuição não geram DANFE nem validação XSD de NF-e.".into());
    }
    let parsed = crate::manifestador::parse_nfe_xml(xml)?;
    if parsed.chave.len() != 44
        || !parsed.chave.bytes().all(|b| b.is_ascii_digit())
        || &parsed.chave[20..22] != "55"
    {
        return Err("Este recurso aceita somente NF-e modelo 55.".into());
    }
    Ok(())
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
    let (content, path) = record.ok_or("NF-e não pertence à empresa ativa.")?;
    let xml = match content.filter(|s| !s.trim().is_empty()) {
        Some(xml) => xml,
        None => std::fs::read_to_string(path.ok_or("Baixe o XML completo antes desta operação.")?)
            .map_err(|_| "Não foi possível ler o XML completo.")?,
    };
    check_xml(&xml)?;
    let mut result = run(&xml, data_dir, operation == "danfe")?;
    if operation == "danfe" {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
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

#[cfg(not(all(
    any(target_os = "windows", target_os = "linux"),
    any(target_arch = "x86", target_arch = "x86_64")
)))]
fn run(_xml: &str, _data_dir: &Path, _pdf: bool) -> Result<Value, String> {
    Err(status()["message"]
        .as_str()
        .unwrap_or("ACBrLib indisponível.")
        .into())
}

#[cfg(all(
    any(target_os = "windows", target_os = "linux"),
    any(target_arch = "x86", target_arch = "x86_64")
))]
fn run(xml: &str, data_dir: &Path, pdf: bool) -> Result<Value, String> {
    native::run(xml, data_dir, pdf)
}

#[cfg(all(
    any(target_os = "windows", target_os = "linux"),
    any(target_arch = "x86", target_arch = "x86_64")
))]
mod native {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use libloading::Library;
    use std::{
        ffi::{c_char, c_void, CString},
        fs,
        path::PathBuf,
        sync::Mutex,
    };
    // MT/cdecl: cada método recebe o handle criado por NFE_Inicializar.
    type Init = unsafe extern "C" fn(*mut *mut c_void, *const c_char, *const c_char) -> i32;
    type Simple = unsafe extern "C" fn(*mut c_void) -> i32;
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char) -> i32;
    type Config =
        unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, *const c_char) -> i32;
    type Output = unsafe extern "C" fn(*mut c_void, *mut c_char, *mut i32) -> i32;
    static LOCK: Mutex<()> = Mutex::new(());
    struct WorkingDir(PathBuf);
    impl Drop for WorkingDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct Instance {
        handle: *mut c_void,
        finalize: Simple,
    }
    impl Drop for Instance {
        fn drop(&mut self) {
            unsafe {
                (self.finalize)(self.handle);
            }
        }
    }
    fn c(s: &str) -> Result<CString, String> {
        CString::new(s).map_err(|_| "Texto contém byte nulo.".into())
    }
    fn configured_path(key: &str, dir: bool) -> Result<PathBuf, String> {
        let path = PathBuf::from(
            std::env::var_os(key)
                .ok_or_else(|| format!("Configure {key} no servidor para usar ACBrLib."))?,
        );
        if !path.is_absolute() || (if dir { !path.is_dir() } else { !path.is_file() }) {
            return Err(format!(
                "{key} deve apontar para um caminho absoluto existente."
            ));
        }
        Ok(path)
    }
    fn output(instance: &Instance, function: Output) -> Result<String, String> {
        let mut buffer = vec![0u8; 8192];
        let mut size = buffer.len() as i32;
        let mut rc = unsafe { function(instance.handle, buffer.as_mut_ptr().cast(), &mut size) };
        if size > buffer.len() as i32 && size <= 1024 * 1024 {
            buffer.resize(size as usize + 1, 0);
            size = buffer.len() as i32;
            rc = unsafe { function(instance.handle, buffer.as_mut_ptr().cast(), &mut size) };
        }
        if rc != 0 || size < 0 || size as usize > buffer.len() {
            return Err("Retorno inválido da ACBrLib.".into());
        }
        let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
        Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }
    fn checked(instance: &Instance, rc: i32, last: Output, action: &str) -> Result<(), String> {
        if rc == 0 {
            Ok(())
        } else {
            Err(format!(
                "ACBrLib: {action} falhou ({rc}): {}",
                output(instance, last).unwrap_or_default()
            ))
        }
    }
    pub fn run(xml: &str, data_dir: &Path, pdf: bool) -> Result<Value, String> {
        let _lock = LOCK
            .lock()
            .map_err(|_| "Motor fiscal indisponível após falha anterior.")?;
        let library_path = configured_path("MANIFESTADOR_ACBR_NFE_MT_CDECL_LIBRARY", false)?;
        let schemas = configured_path("MANIFESTADOR_ACBR_SCHEMAS", true)?;
        if !schemas.join("nfe_v4.00.xsd").is_file() {
            return Err("Pacote de schemas incompleto: nfe_v4.00.xsd ausente.".into());
        }
        let parent = data_dir.join("fiscal-jobs");
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        let job = WorkingDir(parent.join(uuid::Uuid::new_v4().to_string()));
        fs::create_dir(&job.0).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&job.0, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let ini = job.0.join("acbr.ini");
        fs::write(&ini, b"[Principal]\nLogNivel=0\n").map_err(|e| e.to_string())?;
        // SAFETY: somente o operador local define a biblioteca MT/cdecl e sua arquitetura.
        // Símbolos são resolvidos antes de inicializar. O handle não escapa desta chamada,
        // finaliza antes do unload e todos os buffers permanecem vivos durante cada chamada.
        unsafe {
            let lib = Library::new(&library_path)
                .map_err(|e| format!("Não foi possível carregar ACBrLib MT/cdecl: {e}"))?;
            let init: Init = *lib.get(b"NFE_Inicializar\0").map_err(|e| e.to_string())?;
            let finalize: Simple = *lib.get(b"NFE_Finalizar\0").map_err(|e| e.to_string())?;
            let clear: Simple = *lib.get(b"NFE_LimparLista\0").map_err(|e| e.to_string())?;
            let validate: Simple = *lib.get(b"NFE_Validar\0").map_err(|e| e.to_string())?;
            let print: Simple = *lib.get(b"NFE_ImprimirPDF\0").map_err(|e| e.to_string())?;
            let load: Load = *lib.get(b"NFE_CarregarXML\0").map_err(|e| e.to_string())?;
            let config: Config = *lib
                .get(b"NFE_ConfigGravarValor\0")
                .map_err(|e| e.to_string())?;
            let last: Output = *lib.get(b"NFE_UltimoRetorno\0").map_err(|e| e.to_string())?;
            let version: Output = *lib.get(b"NFE_Versao\0").map_err(|e| e.to_string())?;
            let mut handle = std::ptr::null_mut();
            let rc = init(
                &mut handle,
                c(&ini.to_string_lossy())?.as_ptr(),
                c(&uuid::Uuid::new_v4().to_string())?.as_ptr(),
            );
            if rc != 0 || handle.is_null() {
                return Err(format!("Falha ao inicializar ACBrLib MT/cdecl ({rc}). Verifique versão, dependências e ABI."));
            }
            let instance = Instance { handle, finalize };
            let version = output(&instance, version)?;
            for (section, key, value) in [
                ("Principal", "LogNivel", "0"),
                ("NFe", "VersaoDF", "3"),
                ("NFe", "ModeloDF", "0"),
                ("DFe", "SSLXmlSignLib", "4"),
                ("NFe", "ExibirErroSchema", "0"),
                ("NFe", "Visualizar", "0"),
                ("NFe", "SalvarGer", "0"),
                (
                    "NFe",
                    "PathSchemas",
                    schemas
                        .to_str()
                        .ok_or("Caminho de schemas deve ser UTF-8.")?,
                ),
                (
                    "DANFE",
                    "PathPDF",
                    job.0.to_str().ok_or("Caminho de saída deve ser UTF-8.")?,
                ),
                ("DANFE", "UsaSeparadorPathPDF", "0"),
                ("DANFE", "MostraPreview", "0"),
                ("DANFE", "MostraSetup", "0"),
                ("DANFE", "MostraStatus", "0"),
                ("DANFE", "TipoDANFE", "1"),
            ] {
                checked(
                    &instance,
                    config(
                        handle,
                        c(section)?.as_ptr(),
                        c(key)?.as_ptr(),
                        c(value)?.as_ptr(),
                    ),
                    last,
                    "configuração",
                )?;
            }
            checked(&instance, clear(handle), last, "limpeza")?;
            checked(
                &instance,
                load(handle, c(xml)?.as_ptr()),
                last,
                "leitura do XML",
            )?;
            checked(&instance, validate(handle), last, "validação XSD")?;
            if !pdf {
                return Ok(
                    json!({"valid":true,"engine":"ACBrLibNFe","version":version,"message":"XML validado contra os schemas NF-e instalados. Isso não verifica a assinatura nem confirma autorização ou situação atual na SEFAZ."}),
                );
            }
            checked(&instance, print(handle), last, "geração do DANFE PDF")?;
            let files: Vec<_> = fs::read_dir(&job.0)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
                .collect();
            if files.len() != 1 {
                return Err("ACBrLib não produziu exatamente um PDF nesta operação.".into());
            }
            if fs::metadata(&files[0]).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
                return Err("PDF excedeu o limite de 32 MiB.".into());
            }
            let bytes = fs::read(&files[0]).map_err(|e| e.to_string())?;
            if !bytes.starts_with(b"%PDF-") {
                return Err("O arquivo retornado pela ACBrLib não é PDF.".into());
            }
            Ok(
                json!({"engine":"ACBrLibNFe","version":version,"mime_type":"application/pdf","content_base64":STANDARD.encode(bytes),"file_name":"danfe.pdf"}),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_summary_dtd_and_other_company() {
        assert!(check_xml("<resNFe/>").is_err());
        assert!(
            check_xml("<!DOCTYPE NFe [<!ENTITY ext SYSTEM 'file:///etc/passwd'>]><NFe/>").is_err()
        );
        assert!(check_xml("<NFe><infNFe versao='3.10'/></NFe>").is_err());
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE manifestador_notas(id INTEGER,empresa_id INTEGER,xml_content TEXT,xml_path TEXT); INSERT INTO manifestador_notas VALUES(1,1,'<NFe/>',NULL);").unwrap();
        assert!(process(&conn, Path::new("."), 2, 1, "danfe")
            .unwrap_err()
            .contains("não pertence"));
        assert!(process(&conn, Path::new("."), 1, 1, "unknown").is_err());
    }
    #[cfg(all(target_os = "linux", any(target_arch = "x86", target_arch = "x86_64")))]
    #[test]
    fn acbr_mt_contract_and_error_propagation() {
        let dir = std::env::temp_dir().join(format!("acbr-contract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let lib = dir.join("fixture.so");
        let code = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/acbr/contract.c");
        assert!(std::process::Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(code)
            .arg("-o")
            .arg(&lib)
            .status()
            .unwrap()
            .success());
        std::fs::write(dir.join("nfe_v4.00.xsd"), "test double; not a schema").unwrap();
        let lib_key = "MANIFESTADOR_ACBR_NFE_MT_CDECL_LIBRARY";
        let schema_key = "MANIFESTADOR_ACBR_SCHEMAS";
        let old_lib = std::env::var_os(lib_key);
        let old_schema = std::env::var_os(schema_key);
        std::env::set_var(lib_key, &lib);
        std::env::set_var(schema_key, &dir);
        let valid = run("fixture", &dir, false);
        let invalid = run("REJECT", &dir, false);
        let pdf = run("fixture", &dir, true);
        if let Some(v) = old_lib {
            std::env::set_var(lib_key, v)
        } else {
            std::env::remove_var(lib_key)
        };
        if let Some(v) = old_schema {
            std::env::set_var(schema_key, v)
        } else {
            std::env::remove_var(schema_key)
        };
        assert_eq!(valid.unwrap()["version"], "contract-test-only");
        assert!(invalid.unwrap_err().contains("fixture validation error"));
        assert_eq!(pdf.unwrap()["mime_type"], "application/pdf");
        assert_eq!(
            std::fs::read_dir(dir.join("fiscal-jobs")).unwrap().count(),
            0
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[ignore = "Exige ACBrLib MT/cdecl real, schemas e XML de homologação autorizados no ambiente"]
    fn acbr_real_engine_smoke() {
        let input =
            std::env::var("MANIFESTADOR_ACBR_SMOKE_XML").expect("Defina XML NF-e de homologação");
        let xml = std::fs::read_to_string(input).unwrap();
        check_xml(&xml).unwrap();
        let dir = std::env::temp_dir().join(format!("acbr-smoke-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let result = run(&xml, &dir, true);
        std::fs::remove_dir_all(dir).unwrap();
        let pdf = result.expect("ACBrLib real deve validar o XML e gerar PDF");
        assert_ne!(pdf["version"], "contract-test-only");
        assert!(!pdf["content_base64"].as_str().unwrap().is_empty());
    }
}
