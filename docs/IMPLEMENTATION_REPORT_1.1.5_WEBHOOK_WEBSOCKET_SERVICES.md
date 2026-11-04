# Manifestador NF-e Tauri Desktop Web 1.1.5

## Objetivo

Esta versão consolida a arquitetura de serviços nativos do template, adicionando dois serviços genéricos administráveis:

- **Webhook Service** na porta padrão `61003`.
- **WebSocket Service** na porta padrão `61004`.

Também remove definitivamente o recurso de bloqueio de múltiplas instâncias, pois ele estava travando a inicialização em alguns ambientes Windows.

## Portas padrão

| Porta | Serviço | Papel |
|---:|---|---|
| 61001 | API interna | REST, health, logs, OpenAPI e Scalar |
| 61002 | Web App / Proxy | Interface Web da mesma aplicação e proxy `/__internal_api/*` |
| 61003 | Webhook Service | Entrada HTTP genérica de eventos externos |
| 61004 | WebSocket Service | Bridge genérica de eventos em tempo real |

## Multi-instância

O plugin `tauri-plugin-single-instance` foi removido do template.

A aplicação volta a permitir múltiplas instâncias por padrão. Conflitos de porta continuam sendo tratados individualmente pelos serviços com fallback/erro controlado.

## Webhook Service

Rotas principais:

```text
GET  /health
GET  /events
POST /test
POST /webhooks/test
GET  /webhooks/events
POST /webhooks/:provider
POST /webhooks/:provider/:event
```

Comandos Tauri:

```text
webhook_status
webhook_start
webhook_stop
webhook_restart
webhook_list_events
webhook_clear_events
webhook_test_receive
```

## WebSocket Service

Rotas principais:

```text
GET /health
GET /clients
WS  /ws
```

Comandos Tauri:

```text
websocket_status
websocket_start
websocket_stop
websocket_restart
websocket_list_clients
websocket_broadcast_test
```

## Frontend administrativo

Foram adicionadas páginas administrativas:

```text
/#/webhooks
/#/websocket
```

Menu:

```text
Sistema
  Webhooks
  WebSocket
```

## Configuração por ambiente

Principais variáveis:

```env
MANIFESTADOR_NFE_WEBHOOK_ENABLED=false
MANIFESTADOR_NFE_WEBHOOK_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBHOOK_PORT=61003
MANIFESTADOR_NFE_WEBHOOK_BASE_PATH=/webhooks
MANIFESTADOR_NFE_WEBHOOK_TOKEN_REQUIRED=true
MANIFESTADOR_NFE_WEBHOOK_TOKEN_HEADER=X-Webhook-Token
MANIFESTADOR_NFE_WEBHOOK_TOKEN=

MANIFESTADOR_NFE_WEBSOCKET_ENABLED=false
MANIFESTADOR_NFE_WEBSOCKET_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBSOCKET_PORT=61004
MANIFESTADOR_NFE_WEBSOCKET_PATH=/ws
MANIFESTADOR_NFE_WEBSOCKET_TOKEN_REQUIRED=true
MANIFESTADOR_NFE_WEBSOCKET_TOKEN_QUERY=token
MANIFESTADOR_NFE_WEBSOCKET_TOKEN_HEADER=X-WebSocket-Token
MANIFESTADOR_NFE_WEBSOCKET_TOKEN=
```

## Validações executadas nesta entrega

Executado neste ambiente:

```bash
npm ci --ignore-scripts --registry https://registry.npmjs.org/
npm run typecheck
npm run build:web
```

Não foi executado Rust/Tauri neste ambiente porque não há `cargo` instalado.

No Windows, validar com:

```bat
build-windows.bat -Clean -InstallMode Install
```
