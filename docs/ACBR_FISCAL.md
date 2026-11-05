# Motor fiscal embarcado e certificados

O menu **Manifestador → Documentos fiscais** aceita XML completo versão 4.00 dos modelos 55, 65 e 57. **Validar XSD** usa libxml2 estática; **DANFE PDF** valida primeiro e produz PDF A4 para NF-e ou recibo para NFC-e, com o conteúdo do XML. Não requer instalação de ACBrLib, fontes, DLL/SO ou acesso à internet para essas duas operações. O renderer Rust `danfe` e suas fontes são compilados no aplicativo, assim como os schemas e o validador. As antigas variáveis `MANIFESTADOR_ACBR_*` deixam de ser utilizadas.

## Escopo verificável

- XML completo NF-e/NFC-e, isolado ou em `nfeProc`, é importado, armazenado por empresa, validado e convertido em PDF. O XML já armazenado pode ser baixado sem consultar um provedor. Resumos `resNFe` não contêm dados suficientes para PDF; precisam do XML completo.
- A manifestação do destinatário permanece exclusiva de NF-e. A interface e o backend recusam manifestação de NFC-e antes de transmitir ao provedor. O download nativo DistribuiçãoDFe não é utilizado para obter NFC-e: importe seu XML completo.
- CT-e modelo 57, versão 4.00, possui detecção, importação, download do XML armazenado e validação XSD próprios, incluindo schemas dos modais. Não gera DANFE: DACTE está indisponível até haver um renderer apropriado integrado e testado. Não possui distribuição, assinatura ou transmissão CT-e nativas nesta implementação.
- MDF-e, CT-e OS, BP-e, NFCom, NF3e e NFS-e são identificados e recusados explicitamente na importação completa. Outros modelos/versões não são apresentados como NF-e; possuir seu XSD no pacote de referência não equivale a implementar seus fluxos.
- DTD/entidades externas, entradas maiores que 8 MiB, profundidade excessiva, schemas adulterados e documentos que violem o XSD são recusados. O parser XML não acessa rede. Os includes são arquivos locais do pacote incorporado.
- A validação XSD verifica estrutura conforme o pacote incluído; não verifica assinatura criptográfica, autorização, regras fiscais ou situação atual na SEFAZ. O nome/data do pacote não comprova vigência de todas as notas técnicas.
- PDF e XML permanecem vinculados à empresa, com verificação de acesso na API e no desktop. PDFs são salvos em `exports/empresa-<id>` e também devolvidos para download pela interface; o comportamento do download no WebView varia por plataforma.
- Consulta e transmissão a serviços fiscais continuam dependendo de internet, certificado e disponibilidade dos serviços. Esta mudança não acrescenta assinatura nativa de eventos nem emissão fiscal.

## Seleção de certificados

No cadastro fiscal da empresa, **Selecionar arquivo** abre o seletor nativo do navegador/WebView para PFX/P12. O arquivo é copiado para armazenamento privado da aplicação, em diretório da empresa e nome aleatório; em Unix, diretório 0700 e arquivo 0600. Em conexão REST, o arquivo vai para o servidor configurado. Informe a senha e salve a empresa para aplicar a referência. A seleção não valida senha, cadeia ou validade; essa verificação ocorre no uso do transporte existente. Os registros já salvos continuam válidos.

No aplicativo Windows, **Certificados instalados** lista metadados públicos do repositório `CurrentUser/My` com referência de chave privada. A referência selecionada usa `windows-store://<thumbprint>`. O transporte HTTPS WinHTTP usa o certificado sem exportar a chave; valida o servidor e bloqueia redirecionamentos. A enumeração não é exposta pela API REST. A1/A3 dependem de o provedor CSP/CNG estar instalado e disponível para o usuário que executa o aplicativo. PIN permanece sob controle do provedor Windows; não é salvo pela aplicação. Consultas automáticas nativas ignoram referências de certificados instalados, evitando prompts de PIN em segundo plano; use consulta manual.

A3 exige o dispositivo físico e, conforme o fabricante, seu driver/middleware: o aplicativo não consegue incorporá-los universalmente. Não há homologação com token físico nesta alteração. Seleção de repositório macOS/Linux, PKCS#11, certificado remoto em nuvem e TLS com certificado no Android/iOS não foram implementados. O seletor PFX/P12 não implica transporte fiscal nativo em plataformas que ainda não o suportam. PEM avulso não é tratado como PFX.

## Compilação, origem e licenças

O usuário final não instala o motor. A máquina de build precisa de CMake e compilador C, além do toolchain Rust e SDK da plataforma. `src-tauri/fiscal-xml` compila o fonte libxml2 2.15.4 incluído em `vendor`, verifica seu SHA-256 e faz vinculação estática com símbolos internos ocultos em Unix. O pacote não baixa bibliotecas no runtime. O renderer `danfe` 0.1.0 está fixado no Cargo.lock e usa Typst e fontes Liberation incorporadas. A compilação é mais pesada; o CI limita concorrência a dois jobs Cargo.

- libxml2: [GNOME/libxml2](https://github.com/GNOME/libxml2), commit `96498992efa48d52b0e8b83058bd88dbdaf153c1`, licença MIT e avisos adicionais preservados no arquivo fonte. SHA-256 do tar: `14f2d86b5d34f23e8651d07d070ab3c651e8100e2dc8a103b50f18fd2f408499`.
- Renderer: [rust_danfe](https://github.com/juniortada/rust_danfe), MIT/Apache-2.0; distribuição usando os termos MIT. Liberation: SIL Open Font License.
- Schemas NF-e 4.00: arquivos originais, sem modificações, com proveniência e hashes em `src-tauri/resources/fiscal/schemas/nfe/manifest.json`. Atualizações exigem revisão explícita do pacote e dos testes.
- Avisos são distribuídos em `third-party-fiscal-notices.txt` no frontend incorporado e em `src-tauri/resources/licenses` no código-fonte.
- [WinHTTP e certificados de cliente](https://learn.microsoft.com/en-us/windows/win32/winhttp/ssl-in-winhttp).

## Testes e limites da homologação

`cargo test --manifest-path src-tauri/Cargo.toml --locked --no-default-features --lib` executa validação XSD e geração PDF reais para fixtures públicas de homologação dos dois modelos, rejeição de XML inválido/DTD, isolamento entre empresas, importação/download de NFC-e e bloqueio de manifestação inadequada. Não há biblioteca fiscal simulada nesses testes. A origem dos XML está em `tests/fiscal/README.md`.

CT-e também possui teste real de XSD, bloqueio de DANFE/manifestação NF-e e importação por empresa. A consulta por chave é validada contra `distDFeInt_v1.01.xsd`. Uma intenção local sem protocolo não altera mais a situação de manifestação; o último NSU só avança depois da importação completa do lote.

As páginas de PDF geradas foram renderizadas e inspecionadas localmente: NF-e A4 com chave/código de barras, itens e totais; NFC-e com QR, chave, pagamento e total. Isso não substitui homologação fiscal de todos os layouts e cenários. O CI Windows compila o transporte e enumera metadados; não testa um token, senha, PIN ou transação real com a SEFAZ. Builds Android/iOS verificam compilação, não funcionamento de hardware fiscal.

SMTP global e por empresa permanecem preservados. A precedência/fallback segue pendente de definição e não foi alterada.

## Atualização controlada de schemas

A referência é https://svn.code.sf.net/p/acbr/code/trunk2/Exemplos/ACBrDFe/Schemas. O pacote atual veio do arquivo fornecido `Schemas_04112026.zip`; seus hashes e a origem estão nos manifests de cada família. A revisão SVN atual **não foi confirmada**: o proxy deste ambiente recusou o acesso com HTTP 403. O agrupador `cte-package_v4.00.xsd` é gerado pelo projeto para carregar também os tipos dos modais; os demais XSD são preservados byte a byte.

O mantenedor pode executar `python3 scripts/fiscal/update-schemas.py --revision NUMERO` numa máquina autorizada com Subversion instalado. O script exporta somente a revisão explicitamente escolhida, resolve dependências locais, rejeita referências externas/traversal/DTD, registra revisão e hashes, e atualiza o índice compilado. `--verify` confere hashes e dependências offline e roda no CI. O script modifica apenas o checkout: é necessário revisar diff/licenças, testes fiscais e CI antes de integrar. Não há atualização silenciosa, download em runtime nem publicação automática pelo script. A execução de download não foi validada neste ambiente bloqueado.
