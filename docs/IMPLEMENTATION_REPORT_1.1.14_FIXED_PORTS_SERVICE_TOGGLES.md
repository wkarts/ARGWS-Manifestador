# Manifestador NF-e Tauri 1.1.14 — Fixed Ports + Service Toggles

## Objetivo

Corrigir o comportamento do Webport/Proxy para não trocar automaticamente a porta configurada e adicionar parâmetros explícitos para habilitar/desabilitar e iniciar automaticamente os serviços publicados pelo aplicativo.

## Decisões aplicadas

- O Webport/Proxy não tenta mais portas alternativas como `61003`, `61004` ou superiores.
- O Webport/Proxy não faz fallback silencioso para `127.0.0.1` quando configurado para LAN.
- A porta configurada é respeitada. Se estiver ocupada, a aplicação desktop continua abrindo e registra aviso.
- Se a porta configurada já estiver ocupada por outra instância válida do próprio Manifestador NF-e, a instância existente é reutilizada.
- Webhook Service e WebSocket Service também deixam de buscar portas alternativas automáticas.
- Foram adicionados parâmetros persistentes para habilitar/desabilitar e auto iniciar Webport/Proxy, Webhook Service e WebSocket Service.

## Novos parâmetros runtime

```env
MANIFESTADOR_NFE_WEB_ENABLED=true
MANIFESTADOR_NFE_WEB_HOST=0.0.0.0
MANIFESTADOR_NFE_WEB_PORT=61002
MANIFESTADOR_NFE_WEB_AUTO_START=true

MANIFESTADOR_NFE_WEBHOOK_ENABLED=false
MANIFESTADOR_NFE_WEBHOOK_AUTO_START=false
MANIFESTADOR_NFE_WEBHOOK_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBHOOK_PORT=61003

MANIFESTADOR_NFE_WEBSOCKET_ENABLED=false
MANIFESTADOR_NFE_WEBSOCKET_AUTO_START=false
MANIFESTADOR_NFE_WEBSOCKET_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBSOCKET_PORT=61004
```

## Tela Sistema e parâmetros

A tela passou a expor os campos:

- Habilitar Webport/Proxy
- Iniciar Webport/Proxy com a aplicação
- Host/porta Webport/Proxy
- Habilitar Webhook Service
- Iniciar Webhook Service automaticamente
- Host/porta Webhook Service
- Habilitar WebSocket Service
- Iniciar WebSocket Service automaticamente
- Host/porta WebSocket Service

## Arquivos alterados

- `src-tauri/src/runtime_config.rs`
- `src-tauri/src/internal_api/web_server.rs`
- `src-tauri/src/native_webhook/server.rs`
- `src-tauri/src/native_websocket/server.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/cli/runner.rs`
- `src-tauri/src/commands/runtime.rs`
- `src-tauri/src/internal_api/app_commands.rs`
- `src/services/crud.ts`
- `src/pages/SystemPage.vue`
- `src/pages/optional/InternalApiPage.vue`
- `src/core/invoker/WebCommandProvider.ts`
- `.env.example`
- `.env.model`

## Comportamento esperado

Com os parâmetros padrão:

- API interna: `0.0.0.0:61001` quando configurada assim.
- Webport/Proxy: `0.0.0.0:61002`.
- Webhook Service: `0.0.0.0:61003`, somente se habilitado e com auto start/manual start.
- WebSocket Service: `0.0.0.0:61004`, somente se habilitado e com auto start/manual start.

Se `61002` estiver ocupada por processo externo, o Webport não será iniciado em outra porta. A aplicação desktop permanece utilizável, e o usuário deve liberar a porta ou alterar a porta nos parâmetros.
