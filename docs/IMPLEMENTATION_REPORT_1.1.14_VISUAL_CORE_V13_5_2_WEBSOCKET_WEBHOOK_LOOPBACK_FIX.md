# Implementação 1.1.14 — Visual Core v13.5.2 — WebSocket/Webhook Loopback Fix

## Objetivo

Aplicar a mesma proteção de loopback/LAN usada no WebPort 61002 aos serviços nativos WebSocket e Webhook, com foco principal na porta 61004.

## Correções aplicadas

- Corrigido WebSocket Service na porta 61004 quando publicado em IP de rede local, mas inacessível por `localhost`/`127.0.0.1`.
- Adicionado listener auxiliar de loopback para o WebSocket Service quando o bind efetivo fica em IP LAN específico.
- Aplicada a mesma proteção preventiva ao Webhook Service para evitar propagação do mesmo comportamento em outras portas de serviços nativos.
- Mantido o acesso por IP da rede local.
- Mantido o acesso por `localhost`/`127.0.0.1` quando a porta de loopback estiver livre.
- Caso o loopback esteja ocupado por outro processo, o serviço LAN continua ativo e o sistema registra aviso técnico no stderr.
- O listener auxiliar é encerrado junto com o serviço principal.

## Arquivos alterados

- `src-tauri/src/native_websocket/server.rs`
- `src-tauri/src/native_webhook/server.rs`

## Não alterado

- Visual/frontend
- Regras de negócio
- Endpoints
- Payloads
- Porta padrão dos serviços
- API interna
- WebPort 61002 já corrigido anteriormente

## Observação técnica

O problema ocorre quando um serviço fica vinculado a um IP específico da LAN, por exemplo `192.168.x.x:61004`. No Windows, esse bind não cobre automaticamente `127.0.0.1:61004`. A correção adiciona um listener auxiliar no loopback apenas quando necessário.

## Validação

Não foi possível executar `cargo check`/`cargo clippy` neste ambiente porque o Rust/Cargo não está instalado.
