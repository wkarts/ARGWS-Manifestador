# Visual Core v13.5.1 — Clippy Thread Config Fix

## Objetivo

Corrigir falha do `cargo clippy` causada pela regra `clippy::too_many_arguments` na função `spawn_web_server_thread`.

## Correção aplicada

A função `spawn_web_server_thread` deixou de receber 8 argumentos diretos e passou a receber uma estrutura de configuração única:

- `WebServerThreadConfig`

Essa alteração mantém o mesmo comportamento do WebPort/Proxy 61002, incluindo o listener principal e o listener auxiliar de loopback quando aplicável.

## Arquivo alterado

- `src-tauri/src/internal_api/web_server.rs`

## Preservação

- Não altera visual.
- Não altera regra de negócio.
- Não altera endpoints.
- Não altera payloads.
- Não altera porta padrão 61002.
- Não altera a lógica de WebPort LAN/loopback.
- Não adiciona `#[allow(...)]`; corrige a causa do alerta do Clippy.

## Validação

Não foi possível executar `cargo check`/`cargo clippy` neste ambiente porque o binário `cargo` não está instalado.
