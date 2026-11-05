# DANFE e validação XML com ACBr

O menu **Manifestador NF-e** concentra a consulta e as operações de manifestação. Na lista de documentos, **Validar XSD** valida o XML completo com ACBrLibNFe; **DANFE PDF** valida e solicita o PDF à mesma biblioteca. O antigo relatório HTML simplificado não é mais apresentado como DANFE.

A integração exige ACBrLib instalada. Esta alteração **não embute binários ACBr**, e sua homologação com o motor real permanece pendente. Sem configuração, dependências ou XML completo, a operação retorna erro explícito; não produz documento de demonstração nem registra aprovação fictícia. Os testes de contrato usam uma biblioteca artificial identificada como `contract-test-only`, exclusivamente nos testes; ela não faz parte do aplicativo distribuído.

## Instalação no servidor ou desktop

A documentação oficial da ACBrLib informa suporte a Windows/Linux em x86/x64. O adaptador usa exclusivamente a ABI **MT (MultiThread), cdecl**, da mesma arquitetura do processo. Não usar variante ST ou stdcall: os nomes exportados não permitem distinguir essas ABIs com segurança. Uma biblioteca incompatível pode encerrar o processo. Instalação e atualização são tarefas do operador da máquina, não de usuários da API.

Defina no ambiente do processo:

```dotenv
MANIFESTADOR_ACBR_NFE_MT_CDECL_LIBRARY=/opt/manifestador/acbr/libacbrnfe64.so
# Opcional: substituir o pacote de schemas fornecido
MANIFESTADOR_ACBR_SCHEMAS=/opt/manifestador/acbr/Schemas/NFe
```

Os valores são exemplos; use caminhos absolutos reais. No Windows, aponte para a DLL MT/cdecl e para o diretório de schemas. Instale as dependências exigidas pela compilação escolhida, inclusive a biblioteca de manipulação XML utilizada pelo ACBr. `SSLXmlSignLib=4` seleciona LibXml2. Sem `MANIFESTADOR_ACBR_SCHEMAS`, o aplicativo usa os seis XSD NF-e 4.00 extraídos, sem alterações, do pacote de referência já fornecido no repositório. A origem e os SHA-256 estão em `src-tauri/resources/fiscal/schemas/nfe/manifest.json`. Os arquivos e dependências são materializados em diretório identificado pelo hash do conteúdo; alterações inesperadas no cache são recusadas. O nome/data do pacote não é garantia de vigência fiscal.

Quando definido, o diretório próprio deve conter o pacote completo e atualizado de schemas NF-e 4.00, incluindo seus includes/imports; a presença de `nfe_v4.00.xsd` é apenas uma verificação preliminar. O operador deve registrar a revisão do pacote compatível com a vigência dos documentos e notas técnicas aplicáveis. O aplicativo não baixa nem atualiza schemas silenciosamente.

Não há distribuição ACBr nativa validada neste projeto para macOS, Linux ARM64, Android ou iOS. Nesses casos, os comandos fiscais podem ser atendidos por uma instalação Windows/Linux compatível através da conexão REST já existente. Isso não transforma o aplicativo local em um motor ACBr embarcado.

## Comportamento e limites

- Aceita uma NF-e completa modelo 55, versão 4.00; resumo `resNFe`, eventos e outros modelos são recusados neste recurso.
- Rejeita DTD, entidades externas e entrada acima de 8 MiB antes da chamada à biblioteca.
- Valida vínculo/permissão do usuário e pertencimento da nota à empresa, no desktop e na API. Uma operação não altera certificado, NSU, XML original ou situação da manifestação.
- Inicializa uma instância MT por operação; usa diretório temporário exclusivo, finaliza a instância e remove os temporários mesmo em caso de erro. As chamadas são serializadas para evitar concorrência nos componentes de impressão.
- Executa `NFE_CarregarXML`, `NFE_Validar` e, para PDF, `NFE_ImprimirPDF`. Erros da biblioteca são devolvidos ao usuário.
- O resultado XSD corresponde aos schemas instalados. Não comprova assinatura criptográfica, regras de negócio, autorização nem situação atual na SEFAZ.
- O PDF é salvo em `exports/empresa-<id>` no diretório de dados do servidor/desktop, com nome exclusivo. A interface também solicita seu download ao navegador. No WebView, o tratamento de download depende da plataforma; o caminho salvo é informado.
- Consulta, distribuição, manifestação pelo provedor, XML e ZIP existentes permanecem disponíveis. Esta integração não acrescenta assinatura/transmissão nativa de eventos de manifestação, nem declara essas operações homologadas.

## Validação antes do uso

Além do CI, execute com ACBrLib real e XML de homologação autorizado:

```bash
MANIFESTADOR_ACBR_SMOKE_XML=/caminho/nota-homologacao.xml \
  cargo test --manifest-path src-tauri/Cargo.toml --no-default-features \
  --lib --locked acbr_real_engine_smoke -- --ignored
```

Examine o PDF produzido em uma execução pela interface; confira emitente, destinatário, chave, protocolo, itens, tributos, totais, código de barras e paginação. Teste rejeição de um XML que viole os schemas e confirme que nenhum PDF é entregue. O teste de contrato do CI verifica ABI, sequência, propagação de erro e limpeza; não substitui essa homologação fiscal.

## Origem e licença

A implementação usa a interface pública documentada; não copia fontes ACBr nem redistribui DLL/SO nesta PR. Os XSD de referência fornecidos são incorporados sem modificação, preservando seus comentários e avisos de origem. O portal descreve os fontes sob LGPL, e a página de licença da ACBrLib apresenta LGPL 2.1. Antes de distribuir uma compilação ou pacote de schemas, confira os avisos e a versão de licença do artefato específico, preserve atribuições e licença e disponibilize o código-fonte correspondente conforme os termos aplicáveis. A substituição da biblioteca pelo operador permanece possível; não há vinculação estática neste adaptador.

Referências oficiais consultadas:

- [Fontes e SVN do projeto ACBr](https://projetoacbr.com.br/fontes/)
- [ACBrLib e requisitos](https://acbr.sourceforge.io/ACBrLib/BemVindo.html)
- [MT e handles por instância](https://acbr.sourceforge.io/ACBrLib/ACBrLibeMultiThread.html)
- [Convenções de chamada](https://acbr.sourceforge.io/ACBrLib/CdeclouStdCallqualusar.html)
- [Configuração NF-e e DANFE](https://acbr.sourceforge.io/ACBrLib/ConfiguracoesdaBiblioteca16.html)
- [Validação](https://acbr.sourceforge.io/ACBrLib/NFE_Validar.html)
- [PDF](https://acbr.sourceforge.io/ACBrLib/NFE_ImprimirPDF.html)
- [Licença](https://acbr.sourceforge.io/ACBrLib/LicencadeUso.html)
