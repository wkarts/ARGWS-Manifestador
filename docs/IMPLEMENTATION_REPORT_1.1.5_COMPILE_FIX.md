# Correção de compilação Rust - Manifestador NF-e Tauri Desktop Web 1.1.5

## Objetivo

Corrigir falhas de compilação reportadas no build Windows da versão 1.1.5 após a inclusão dos serviços nativos genéricos de Webhook e WebSocket.

## Correções aplicadas

### `src-tauri/src/runtime_config.rs`

Corrigida a função `write_env_file(...)`, que possuía argumentos adicionais para Webhook, WebSocket e tray sem os respectivos placeholders no `format!`.

Agora o `.env` gerado pela tela de Sistema/Parâmetros grava corretamente:

- API interna
- Web app/proxy
- portas auxiliares
- Webhook Service
- WebSocket Service
- tray
- startup/serviços

### `src-tauri/src/native_webhook/routes.rs`

Corrigida a montagem de `AppLogInput` para seguir a assinatura real da struct:

- `source: Some("native_webhook")`
- `route: Some("-")`
- `details: Some(&json!(record))`

Foram removidos campos inexistentes:

- `user_login`
- `details_json`

## Validações executadas neste ambiente

- `npm ci --ignore-scripts --registry https://registry.npmjs.org/`
- `npm run typecheck`
- `npm run build:web`

## Validação pendente no Windows

A validação Rust/Tauri deve ser executada no Windows com cargo instalado:

```bat
build-windows.bat -Clean -InstallMode Install
```

ou:

```bat
npm run build:windows
```

