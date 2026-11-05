# Relatório de implementação — manifestador-nfe 1.1.1

## Status

Versão corretiva conservadora gerada para preservar a base anterior do template e completar a arquitetura Web/PWA/Tauri sem remover compatibilidade.

## Correções sobre a 1.1.0

- Restaurada a camada `src/core/storage/`.
- Restaurado `src/core/storage/WebLocalDatabase.ts` para compatibilidade com a primeira geração desktop-web.
- Adicionado `StorageProvider.ts`.
- Adicionado `WebLocalStorageProvider.ts`.
- Adicionado `WebSessionStorageProvider.ts`.
- Adicionado `StorageProviderFactory.ts`.
- Adicionado `LocalStorageDatabaseProvider.ts`.
- Atualizado `DatabaseProviderFactory.ts` para usar IndexedDB quando disponível e LocalStorage como fallback.
- Atualizada tela/rotina de diagnóstico para mostrar IndexedDB, LocalStorage e SessionStorage.
- Mantida compatibilidade com `src/services/tauri.ts`.
- Mantido o padrão: telas e módulos não importam Tauri diretamente.

## Arquitetura preservada

O template segue com um único projeto, um único visual e uma única base de telas para:

- Web no navegador;
- PWA;
- Tauri Desktop.

Recursos dependentes do ambiente passam por providers:

- RuntimeProvider;
- CommandProvider;
- DatabaseProvider;
- StorageProvider;
- FileProvider;
- NotificationProvider;
- PrintProvider.

## Validações executadas

Executado com sucesso:

```bash
npm ci --ignore-scripts
npm run ci:version
npm run typecheck
npm run build:web
```

Resultado: aprovado.

## Validação Tauri/Rust

Não foi possível executar validação Rust/Tauri neste ambiente porque `cargo` não está instalado.

Comandos recomendados no ambiente de desenvolvimento/release:

```bash
cargo --version
npm run fmt:rust:check
npm run lint:rust
npm run test:rust
npm run tauri:dev
npm run tauri:build
```

## Auditoria de imports Tauri

Resultado da auditoria no frontend:

```text
src/core/invoker/TauriCommandProvider.ts importa @tauri-apps/api/core
```

Nenhum outro arquivo em `src/` importa diretamente `@tauri-apps`.

## Branch semântica

```text
fix/template-desktop-web-storage-compat
```

## Título do PR

```text
fix(template): preserve storage compatibility in desktop-web runtime
```

## Descrição do PR

```text
Este PR corrige a evolução `manifestador-nfe` para preservar a compatibilidade com a camada de storage local anterior, restaurando `src/core/storage`, `WebLocalDatabase` e adicionando providers formais de LocalStorage e SessionStorage.

A implementação mantém IndexedDB como provider principal para Web/PWA, mas adiciona fallback conservador via LocalStorage, evitando remoções abruptas e mantendo o template compatível com a versão anterior.

Também atualiza o diagnóstico do runtime e valida o build Web/TypeScript, mantendo o princípio de um único projeto com o mesmo visual para Web, PWA e Tauri.
```
