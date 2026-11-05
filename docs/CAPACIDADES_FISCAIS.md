# Capacidades fiscais e limites verificados

Esta matriz descreve implementação e evidências, não equivalência integral nem homologação SEFAZ.

| Recurso | Estado atual | Limite/pendência |
|---|---|---|
| Empresa, certificado e parâmetros operacionais | Escopo por empresa, persistência/migração e permissões testados | SMTP global não tem fallback definido |
| NF-e 55 / NFC-e 65 XML 4.00 | Importação, XML local, XSD e PDF embarcados testados | XSD não comprova assinatura/autorização |
| CT-e 57 XML 4.00 | Detecção, importação e XSD próprios testados | DACTE e serviços CT-e não implementados |
| CT-e OS 67, MDF-e 58, BP-e 63, NFCom 62, NF3e 66, NFS-e | Reconhecimento de família e erro explícito | Sem importação completa, validação/PDF/transporte próprios |
| Distribuição NF-e por último NSU | SOAP nativo desktop implementado; cursor por empresa avança após importação integral | Sem teste SEFAZ real; uma página por chamada, sem equivalência ao esgotamento de múltiplos lotes |
| Consulta NF-e por chave | Corrigida para `distDFeInt/consChNFe`, com CPF/CNPJ e UF; XML da requisição validado por XSD | Disponibilidade do XML depende da SEFAZ, vínculo e regras de acesso |
| Consulta por NSU específico | Construtor do protocolo preparado | Sem comando/interface de consulta dedicada; não considerar recurso entregue |
| Resumos e eventos DF-e | `resNFe`/resumos tratados separadamente de XML completo | Não há tratamento completo de eventos distribuídos; erro impede avanço do cursor, evitando perda silenciosa |
| Manifestação NF-e | Quatro intenções e integração por adaptador existentes; sem protocolo, somente histórico local | Falta assinatura XML e envio nativo com tratamento fiscal completo de retornos/duplicidade |
| Manifestação automática | Não implementada como envio fiscal nativo | Não ligar automaticamente sem assinatura, retorno e autorização operacional |
| NFC-e/CT-e e outros eventos | Bloqueados no fluxo de manifestação NF-e | Cada modelo exige seus próprios serviços/eventos; distribuição não concede autorização para manifestação |
| Certificado por arquivo | Seletor PFX/P12, armazenamento por empresa; TLS A1 desktop existente | Sem teste com certificado real nesta entrega; PEM e certificados de nuvem não implementados |
| Certificados instalados Windows | Metadados públicos, referência CSP/CNG, WinHTTP sem exportar chave | Token A3/driver/PIN não testados; consultas manuais; não extensível automaticamente aos outros sistemas |
| XML/ZIP/SMTP/histórico/filtros | Recursos existentes preservados | Envio SMTP/fiscal real não executado nesta validação |
| Impressão física | PDF disponível para os modelos suportados | Não há homologação de impressoras nem fluxo nativo completo de impressão |

## Escolha de motor e build próprio

ACBrLib documenta componentes separados para NFe/NFCe, CTe e MDFe, com validação, assinatura, eventos e documentos auxiliares. Essa cobertura é mais ampla que o motor de PDF/XSD embarcado desta PR. A documentação apresenta binários Windows/Linux i386/x86_64; isso não comprova macOS, ARM64, Android, iOS ou A3 multiplataforma. O método CT-e por chave aparece documentado como aguardando liberação da SEFAZ e não deve ser anunciado disponível apenas por existir uma função.

Uma compilação própria requer obter e fixar uma revisão completa do trunk, versões de Lazarus/Free Pascal, projetos/pacotes e dependências nativas, compilar/testar cada módulo/ABI/alvo e registrar hashes, licenças e fontes correspondentes. A licença LGPL e avisos precisam ser atendidos na distribuição; não se presume autorização de redistribuir binários Pro. Sem o fonte acessível e sem build/teste por alvo, essa alternativa permanece **não validada**, não um recurso instalado.

Neste ambiente, o acesso ao trunk foi recusado pelo proxy (403), não há `fpc`/`lazbuild` instalado e o arquivo de referência não contém os projetos ACBrLib para compilação. Não foi possível produzir uma ACBrLib própria nem comprovar paridade de transporte, assinatura e impressão. O motor Rust/libxml2 desta PR atende os recursos explicitados acima e não é declarado substituto completo do ACBr.

Fontes técnicas:

- https://acbr.sourceforge.io/ACBrLib/BemVindo.html
- https://acbr.sourceforge.io/ACBrLib/MetodosNFe.html
- https://acbr.sourceforge.io/ACBrLib/MetodosCTe.html
- https://acbr.sourceforge.io/ACBrLib/MetodosMDFe.html
- https://svn.code.sf.net/p/acbr/code/trunk2/Exemplos/ACBrDFe/Schemas
