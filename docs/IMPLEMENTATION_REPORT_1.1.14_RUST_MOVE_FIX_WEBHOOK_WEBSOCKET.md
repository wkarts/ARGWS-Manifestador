# Patch 1.1.14 — Rust move fix Webhook/WebSocket

## Objetivo

Corrigir erro de compilação `E0382: borrow of moved value: config` causado pelo envio da configuração para dentro das threads dos serviços nativos de Webhook e WebSocket.

## Causa

As funções `start_background_with_config` usavam `config` dentro de closures `move || { ... }` e, depois do `spawn`, ainda tentavam usar a mesma variável para montar o runtime handle (`bind_host`, `port`, `path`, `base_path`).

Como `WebhookConfig` e `WebSocketConfig` não implementam `Copy`, o valor era movido para a closure e não podia ser reutilizado fora dela.

## Correção

Foi criada uma cópia explícita antes do `spawn`:

```rust
let thread_config = config.clone();
```

A thread passa a receber `thread_config`, preservando a variável `config` original para montar o handle do serviço.

## Arquivos alterados

- `src-tauri/src/native_webhook/server.rs`
- `src-tauri/src/native_websocket/server.rs`

## Impacto

- Não altera regras de porta.
- Não altera splash screen.
- Não altera layout.
- Não altera comportamento de bind.
- Apenas corrige compilação Rust/Clippy.
