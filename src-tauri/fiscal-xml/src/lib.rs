//! XSD 1.0 using statically bundled libxml2 2.15.4. No system DLL/SO or network.
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    path::Path,
    ptr,
};
#[repr(C)]
struct ErrorPrefix {
    domain: c_int,
    code: c_int,
    message: *const c_char,
}
type ErrorHandler = unsafe extern "C" fn(*mut c_void, *const ErrorPrefix);
extern "C" {
    fn xmlNewParserCtxt() -> *mut c_void;
    fn xmlFreeParserCtxt(ctx: *mut c_void);
    fn xmlCtxtReadMemory(
        ctx: *mut c_void,
        bytes: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut c_void;
    fn xmlCtxtGetLastError(ctx: *mut c_void) -> *const ErrorPrefix;
    fn xmlFreeDoc(doc: *mut c_void);
    fn xmlSchemaNewParserCtxt(path: *const c_char) -> *mut c_void;
    fn xmlSchemaFreeParserCtxt(ctx: *mut c_void);
    fn xmlSchemaSetParserStructuredErrors(
        ctx: *mut c_void,
        handler: Option<ErrorHandler>,
        data: *mut c_void,
    );
    fn xmlSchemaParse(ctx: *mut c_void) -> *mut c_void;
    fn xmlSchemaFree(schema: *mut c_void);
    fn xmlSchemaNewValidCtxt(schema: *mut c_void) -> *mut c_void;
    fn xmlSchemaFreeValidCtxt(ctx: *mut c_void);
    fn xmlSchemaSetValidStructuredErrors(
        ctx: *mut c_void,
        handler: Option<ErrorHandler>,
        data: *mut c_void,
    );
    fn xmlSchemaValidateDoc(ctx: *mut c_void, doc: *mut c_void) -> c_int;
}
struct Owned(*mut c_void, unsafe extern "C" fn(*mut c_void));
impl Owned {
    fn new(p: *mut c_void, drop: unsafe extern "C" fn(*mut c_void)) -> Result<Self, String> {
        if p.is_null() {
            Err("Falha ao inicializar o validador XML embarcado.".into())
        } else {
            Ok(Self(p, drop))
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { (self.1)(self.0) }
    }
}
unsafe extern "C" fn errors(data: *mut c_void, error: *const ErrorPrefix) {
    if data.is_null() || error.is_null() {
        return;
    }
    let messages = &mut *(data as *mut Vec<String>);
    if !(*error).message.is_null() && messages.len() < 30 {
        messages.push(
            CStr::from_ptr((*error).message)
                .to_string_lossy()
                .trim()
                .to_owned(),
        );
    }
}
/// Validates one document against a trusted, local schema package.
/// Caller must reject DTD and provide schemas with only local, trusted imports.
pub fn validate(xml: &str, schema: &Path) -> Result<(), String> {
    if xml.len() > 8 * 1024 * 1024 || xml.as_bytes().contains(&0) {
        return Err("XML inválido ou maior que 8 MiB.".into());
    }
    let path = CString::new(schema.to_str().ok_or("Caminho XSD inválido.")?)
        .map_err(|_| "Caminho XSD inválido.")?;
    let mut messages = Vec::<String>::new();
    // SAFETY: all opaque handles are checked, uniquely owned and released in reverse
    // order. Per-context synchronous callbacks borrow messages only for each call.
    // NONET and NO_XXE disable external entity fetching; no global callbacks or cleanup.
    unsafe {
        let parser = Owned::new(xmlNewParserCtxt(), xmlFreeParserCtxt)?;
        let raw = xmlCtxtReadMemory(
            parser.0,
            xml.as_ptr().cast(),
            xml.len() as c_int,
            ptr::null(),
            ptr::null(),
            (1 << 11) | (1 << 23),
        );
        if raw.is_null() {
            errors(
                (&mut messages as *mut Vec<String>).cast(),
                xmlCtxtGetLastError(parser.0),
            );
            return Err(format!("XML malformado: {}", messages.join("; ")));
        }
        let doc = Owned::new(raw, xmlFreeDoc)?;
        let schema_parser = Owned::new(
            xmlSchemaNewParserCtxt(path.as_ptr()),
            xmlSchemaFreeParserCtxt,
        )?;
        xmlSchemaSetParserStructuredErrors(
            schema_parser.0,
            Some(errors),
            (&mut messages as *mut Vec<String>).cast(),
        );
        let schema_raw = xmlSchemaParse(schema_parser.0);
        if schema_raw.is_null() {
            return Err(format!("Falha no pacote XSD: {}", messages.join("; ")));
        }
        let schema = Owned::new(schema_raw, xmlSchemaFree)?;
        let validator = Owned::new(xmlSchemaNewValidCtxt(schema.0), xmlSchemaFreeValidCtxt)?;
        xmlSchemaSetValidStructuredErrors(
            validator.0,
            Some(errors),
            (&mut messages as *mut Vec<String>).cast(),
        );
        match xmlSchemaValidateDoc(validator.0, doc.0) {
            0 => Ok(()),
            code => Err(format!(
                "XML não aprovado nos schemas ({code}): {}",
                messages.join("; ")
            )),
        }
    }
}
