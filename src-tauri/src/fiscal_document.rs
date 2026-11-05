//! Explicit fiscal document routing. Unknown models never fall back to NF-e.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Nfe,
    Nfce,
    Cte,
}
impl Kind {
    pub fn model(self) -> u16 {
        match self {
            Self::Nfe => 55,
            Self::Nfce => 65,
            Self::Cte => 57,
        }
    }
}
pub struct Document {
    pub kind: Kind,
    pub key: String,
    pub processed: bool,
}
pub fn identify(xml: &str) -> Result<Document, String> {
    use quick_xml::events::Event;
    if xml.is_empty() || xml.len() > 8 * 1024 * 1024 || xml.as_bytes().contains(&0) {
        return Err("XML inválido ou maior que 8 MiB.".into());
    }
    let mut reader = quick_xml::Reader::from_str(xml);
    let (mut root, mut key, mut version) = (String::new(), String::new(), String::new());
    let (mut depth, mut info, mut bodies) = (0usize, 0, 0);
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("XML inválido: {e}"))?
        {
            Event::DocType(_) => return Err("DTD e entidades externas não são permitidas.".into()),
            Event::Start(e) => {
                depth += 1;
                if depth > 128 {
                    return Err("XML excede a profundidade permitida.".into());
                }
                let name = e.local_name();
                let n = name.as_ref();
                if root.is_empty() {
                    root = String::from_utf8_lossy(n).into_owned();
                }
                if matches!(n, b"NFe" | b"CTe") {
                    bodies += 1;
                }
                if (n == b"infNFe" && matches!(root.as_str(), "NFe" | "nfeProc"))
                    || (n == b"infCte" && matches!(root.as_str(), "CTe" | "cteProc"))
                {
                    info += 1;
                    for attr in e.attributes() {
                        let a = attr.map_err(|e| e.to_string())?;
                        match a.key.as_ref() {
                            b"Id" => {
                                key = String::from_utf8_lossy(&a.value)
                                    .trim_start_matches("NFe")
                                    .trim_start_matches("CTe")
                                    .to_owned()
                            }
                            b"versao" => version = String::from_utf8_lossy(&a.value).into_owned(),
                            _ => {}
                        }
                    }
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Eof => break,
            _ => {}
        }
    }
    let unsupported = match root.as_str() {
        "CTeOS" | "cteOSProc" => Some("CT-e OS"),
        "MDFe" | "mdfeProc" => Some("MDF-e"),
        "BPe" | "bpeProc" => Some("BP-e"),
        "NFCom" | "nfcomProc" => Some("NFCom"),
        "NF3e" | "nf3eProc" => Some("NF3e"),
        "NFSe" | "CompNfse" | "Nfse" => Some("NFS-e"),
        _ => None,
    };
    if let Some(label) = unsupported {
        return Err(format!("{label} reconhecido, mas importação, schemas e documento auxiliar ainda não estão implementados para esse modelo."));
    }
    if !matches!(root.as_str(), "NFe" | "nfeProc" | "CTe" | "cteProc") || info != 1 || bodies != 1 {
        return Err("Informe XML completo de NF-e, NFC-e ou CT-e. Resumos/eventos não são documentos completos.".into());
    }
    if version != "4.00" {
        return Err("Versão fiscal não suportada neste recurso; esperado leiaute 4.00.".into());
    }
    if key.len() != 44 || !key.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("Chave de acesso inválida.".into());
    }
    let kind = match (&key[20..22], root.as_str()) {
        ("55", "NFe" | "nfeProc") => Kind::Nfe,
        ("65", "NFe" | "nfeProc") => Kind::Nfce,
        ("57", "CTe" | "cteProc") => Kind::Cte,
        _ => return Err("Modelo da chave incompatível com o XML ou ainda não suportado.".into()),
    };
    Ok(Document {
        kind,
        key,
        processed: root.ends_with("Proc"),
    })
}
