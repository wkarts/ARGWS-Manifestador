# Checkpoint — Manifestador NF-e Tauri/Rust 0.2.0

## Base

- Reconstrução nova baseada na template Tauri 2 + Rust + Vue 3 + TypeScript.
- Identidade, ícones, metadados e comandos substituídos para Manifestador NF-e.
- Código Delphi preservado apenas em `legacy-reference/` como referência de migração.

## Correções desta rodada

- `quick-xml` 0.41: `normalized_value(XmlVersion::Implicit1_0)` nos atributos XML.
- `quick-xml` 0.41: `BytesText::decode()` no conteúdo textual.
- Correção do delimitador/retorno em `distribution_summary_from_xml`, identificado pelo `cargo fmt` do build Windows.
- Correção do segundo ciclo de Clippy: uso de `value_string`, variável `uf` não utilizada e vida útil do `Statement` no timer automático.
- Correção do terceiro ciclo de Clippy: `collapsible_match`, closure redundante, enum SMTP com TLS em `Box`, complexidade de tipos, argumentos SMTP, `as_bytes()` desnecessário e referências mutáveis no download.
- Correção do quarto ciclo de Clippy: restauração de `mut` somente nos fluxos que importam XML/documentos e remoção de `mut` no fluxo de download.
- Fallback nativo desktop A1/PFX para `DistribuicaoDFe` e download de XML processado.
- Conversão de `docZip`, `resNFe` e resumos para o modelo SQLite local.
- Mesmo comportamento nos comandos Tauri e na API interna autenticada.
- Consulta automática com NSU persistido e tentativa de download automático.
- Download manual via provedor externo, integração nativa ou XML local.
- `build-windows.bat` com `-ExecutionPolicy Bypass` aplicado apenas ao processo.

## Validações executadas

- `npm run ci:version` — aprovado.
- `npm run typecheck` — aprovado.
- `npm run build:web` — aprovado; apenas avisos já existentes de imports dinâmicos do Vite.
- `python3 scripts/ci/validate-tauri-project.py --project . --mode static` — aprovado.
- `node --test scripts/release/tests/*.test.mjs` — 10 testes aprovados.
- `npm run build:web:tauri` — aprovado; recursos web preparados em `src-tauri/dist`.
- `verify_identity.py` — aprovado, sem resíduos textuais ou visuais da template.
- `Cargo.toml`/`Cargo.lock` — dependências diretas conferidas; lockfile sincronizado em 0.2.0.
- Parser Rust aplicado aos 82 arquivos `.rs` — sem erros sintáticos.

## Validação pendente no ambiente Windows

O ambiente desta sessão não possui `cargo`, `rustc` ou `rustfmt`. Execute na raiz:

```bat
build-windows.bat
```

O script executará automaticamente `cargo fmt`, Clippy, testes Rust e o build/empacotamento Tauri Windows.
