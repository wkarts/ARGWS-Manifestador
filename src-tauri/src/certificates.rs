//! Certificate selection/import. Private keys and PINs never enter list responses.
use serde_json::{json, Value};
use std::path::Path;
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn import_file(
    data_dir: &Path,
    company: i64,
    file_name: &str,
    certificate_secret_base64: &str,
) -> Result<Value, String> {
    if company <= 0 {
        return Err("Selecione uma empresa.".into());
    }
    let extension = Path::new(file_name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "pfx" | "p12") {
        return Err("Selecione um certificado PKCS#12 (.pfx ou .p12).".into());
    }
    if certificate_secret_base64.len() > 1_400_000 {
        return Err("Certificado maior que o limite de 1 MiB.".into());
    }
    let bytes = STANDARD
        .decode(certificate_secret_base64)
        .map_err(|_| "Arquivo de certificado inválido.")?;
    // This checks the container framing, not validity, private-key possession or password.
    if bytes.len() > 1024 * 1024 || bytes.first() != Some(&0x30) {
        return Err("O arquivo não possui formato DER/PKCS#12 válido.".into());
    }
    let dir = data_dir
        .join("certificates")
        .join(format!("empresa-{company}"));
    std::fs::create_dir_all(&dir)
        .map_err(|_| "Não foi possível preparar o armazenamento do certificado.")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Falha ao proteger diretório de certificados.")?;
    }
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4(), extension));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|_| "Não foi possível armazenar o certificado.")?;
    use std::io::Write;
    file.write_all(&bytes)
        .map_err(|_| "Falha ao gravar certificado.")?;
    Ok(
        json!({"path":path.to_string_lossy(),"message":"Arquivo copiado para o armazenamento privado da aplicação. Informe a senha e salve a empresa. A validade será verificada ao utilizar o certificado."}),
    )
}

pub fn installed() -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        windows::list()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Seleção de certificados instalados disponível no aplicativo Windows. A1 por arquivo PFX/P12 continua disponível.".into())
    }
}

#[cfg(target_os = "windows")]
pub fn post_with_installed(
    endpoint: &str,
    thumbprint: &str,
    body: &str,
    timeout: u64,
) -> Result<String, String> {
    windows::post(endpoint, thumbprint, body, timeout)
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::{ffi::c_void, ptr};
    use windows_sys::Win32::{
        Security::Cryptography::*, Networking::WinHttp::*, Foundation::GetLastError,
    };
    struct Store(HCERTSTORE);
    impl Drop for Store {
        fn drop(&mut self) {
            unsafe {
                CertCloseStore(self.0, 0);
            }
        }
    }
    struct Cert(*const CERT_CONTEXT);
    impl Drop for Cert {
        fn drop(&mut self) {
            unsafe {
                CertFreeCertificateContext(self.0);
            }
        }
    }
    struct Http(*mut c_void);
    impl Drop for Http {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn last(action: &str) -> String {
        format!(
            "{action}: erro Windows {}. Verifique certificado, dispositivo e driver do fabricante.",
            unsafe { GetLastError() }
        )
    }
    fn http(p: *mut c_void) -> Result<Http, String> {
        if p.is_null() {
            Err(last("Falha no transporte HTTPS"))
        } else {
            Ok(Http(p))
        }
    }
    fn ok(value: i32, action: &str) -> Result<(), String> {
        if value == 0 {
            Err(last(action))
        } else {
            Ok(())
        }
    }
    fn open() -> Result<Store, String> {
        let p = unsafe { CertOpenSystemStoreW(0, wide("MY").as_ptr()) };
        if p.is_null() {
            Err(last("Falha ao abrir certificados do usuário Windows"))
        } else {
            Ok(Store(p))
        }
    }
    unsafe fn thumb(cert: *const CERT_CONTEXT) -> Option<String> {
        let mut bytes = [0u8; 20];
        let mut size = 20;
        if CertGetCertificateContextProperty(
            cert,
            CERT_SHA1_HASH_PROP_ID,
            bytes.as_mut_ptr().cast(),
            &mut size,
        ) == 0
            || size != 20
        {
            return None;
        }
        Some(bytes.iter().map(|b| format!("{b:02X}")).collect())
    }
    unsafe fn has_private_key(cert: *const CERT_CONTEXT) -> bool {
        let mut size = 0;
        CertGetCertificateContextProperty(
            cert,
            CERT_KEY_PROV_INFO_PROP_ID,
            ptr::null_mut(),
            &mut size,
        ) != 0
            && size > 0
    }
    pub fn list() -> Result<Value, String> {
        let store = open()?;
        let mut items = Vec::new();
        let mut previous = ptr::null_mut();
        // Enumeration releases the previous context. Only public metadata is copied.
        unsafe {
            loop {
                previous = CertEnumCertificatesInStore(store.0, previous);
                if previous.is_null() {
                    break;
                }
                if !has_private_key(previous) {
                    continue;
                }
                let Some(id) = thumb(previous) else {
                    continue;
                };
                let length = CertGetNameStringW(
                    previous,
                    CERT_NAME_SIMPLE_DISPLAY_TYPE,
                    0,
                    ptr::null(),
                    ptr::null_mut(),
                    0,
                );
                let mut name = vec![0u16; length as usize];
                if length > 0 {
                    CertGetNameStringW(
                        previous,
                        CERT_NAME_SIMPLE_DISPLAY_TYPE,
                        0,
                        ptr::null(),
                        name.as_mut_ptr(),
                        length,
                    );
                }
                let label = String::from_utf16_lossy(&name[..name.len().saturating_sub(1)]);
                items.push(json!({"thumbprint":id,"name":label,"valid_now":CertVerifyTimeValidity(ptr::null(),(*previous).pCertInfo)==0}));
            }
        }
        Ok(
            json!({"certificates":items,"store":"CurrentUser/My","message":"A1/A3 com chave privada registrada no Windows. A3 exige dispositivo/driver e poderá solicitar PIN pelo provedor do sistema. O aplicativo não armazena PIN."}),
        )
    }
    fn find(id: &str) -> Result<Cert, String> {
        if id.len() != 40 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Identificador do certificado instalado inválido.".into());
        }
        let store = open()?;
        let mut previous = ptr::null_mut();
        unsafe {
            loop {
                previous = CertEnumCertificatesInStore(store.0, previous);
                if previous.is_null() {
                    break;
                }
                if thumb(previous).is_some_and(|v| v.eq_ignore_ascii_case(id)) {
                    let result = if !has_private_key(previous) {
                        Err("Certificado sem referência de chave privada.".into())
                    } else if CertVerifyTimeValidity(ptr::null(), (*previous).pCertInfo) != 0 {
                        Err("Certificado expirado ou fora do período de validade.".into())
                    } else {
                        let copy = CertDuplicateCertificateContext(previous);
                        if copy.is_null() {
                            Err(last("Falha ao selecionar certificado"))
                        } else {
                            Ok(Cert(copy))
                        }
                    };
                    CertFreeCertificateContext(previous);
                    return result;
                }
            }
        }
        Err("Certificado não encontrado no usuário Windows atual. Reabra a seleção no dispositivo que fará a consulta.".into())
    }
    pub fn post(endpoint: &str, id: &str, body: &str, timeout: u64) -> Result<String, String> {
        let url = url::Url::parse(endpoint).map_err(|_| "URL inválida.")?;
        if url.scheme() != "https" || url.host_str().is_none() {
            return Err("Certificado instalado exige endpoint HTTPS.".into());
        }
        let cert = find(id)?;
        let mut path = url.path().to_owned();
        if let Some(query) = url.query() {
            path.push('?');
            path.push_str(query);
        }
        // Windows handles non-exportable CSP/CNG keys and any provider PIN prompt.
        // No certificate validation bypass or redirect forwarding is enabled.
        unsafe {
            let session = http(WinHttpOpen(
                wide("Manifestador-NFe/1").as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            ))?;
            let milliseconds = (timeout.clamp(5, 300) * 1000) as i32;
            ok(
                WinHttpSetTimeouts(
                    session.0,
                    milliseconds,
                    milliseconds,
                    milliseconds,
                    milliseconds,
                ),
                "Falha ao definir timeout",
            )?;
            let connection = http(WinHttpConnect(
                session.0,
                wide(url.host_str().unwrap()).as_ptr(),
                url.port_or_known_default().unwrap_or(443),
                0,
            ))?;
            let request = http(WinHttpOpenRequest(
                connection.0,
                wide("POST").as_ptr(),
                wide(&path).as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                WINHTTP_FLAG_SECURE,
            ))?;
            let policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
            ok(
                WinHttpSetOption(
                    request.0,
                    WINHTTP_OPTION_REDIRECT_POLICY,
                    (&policy as *const u32).cast(),
                    4,
                ),
                "Falha ao bloquear redirecionamento",
            )?;
            ok(
                WinHttpSetOption(
                    request.0,
                    WINHTTP_OPTION_CLIENT_CERT_CONTEXT,
                    cert.0.cast(),
                    std::mem::size_of::<CERT_CONTEXT>() as u32,
                ),
                "Falha ao selecionar certificado TLS",
            )?;
            let headers = wide("Content-Type: application/soap+xml; charset=utf-8\r\n");
            ok(
                WinHttpSendRequest(
                    request.0,
                    headers.as_ptr(),
                    (headers.len() - 1) as u32,
                    body.as_ptr().cast(),
                    body.len() as u32,
                    body.len() as u32,
                    0,
                ),
                "Falha no envio com certificado instalado",
            )?;
            ok(
                WinHttpReceiveResponse(request.0, ptr::null_mut()),
                "Falha na resposta HTTPS",
            )?;
            let (mut code, mut size) = (0u32, 4u32);
            ok(
                WinHttpQueryHeaders(
                    request.0,
                    WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                    ptr::null(),
                    (&mut code as *mut u32).cast(),
                    &mut size,
                    ptr::null_mut(),
                ),
                "Falha no status HTTPS",
            )?;
            if !(200..300).contains(&code) {
                return Err(format!("Serviço fiscal respondeu HTTP {code}."));
            }
            let mut result = Vec::new();
            let mut buffer = [0u8; 65536];
            loop {
                let mut read = 0;
                ok(
                    WinHttpReadData(
                        request.0,
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        &mut read,
                    ),
                    "Falha ao ler resposta HTTPS",
                )?;
                if read == 0 {
                    break;
                }
                if result.len() + read as usize > 16 * 1024 * 1024 {
                    return Err("Resposta fiscal maior que 16 MiB.".into());
                }
                result.extend_from_slice(&buffer[..read as usize]);
            }
            String::from_utf8(result).map_err(|_| "Resposta fiscal não é UTF-8.".into())
        }
    }
    #[cfg(test)]
    mod tests {
        #[test]
        fn lists_public_metadata_without_exporting_keys() {
            let result = super::list().unwrap();
            assert!(result["certificates"].is_array());
            assert!(!result.to_string().contains("PRIVATE KEY"));
        }
        #[test]
        fn rejects_invalid_store_reference() {
            assert!(super::find("not-a-thumbprint").is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_import_and_format_limits() {
        let dir = std::env::temp_dir().join(format!("cert-test-{}", uuid::Uuid::new_v4()));
        assert!(import_file(&dir, 1, "x.txt", "MAA=").is_err());
        assert!(import_file(&dir, 0, "x.pfx", "MAA=").is_err());
        let result = import_file(&dir, 1, "../../cert.p12", "MAA=").unwrap();
        let path = Path::new(result["path"].as_str().unwrap());
        assert!(path.starts_with(dir.join("certificates/empresa-1")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(result.get("certificate_secret_base64").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
