# Manifestador NF-e Tauri 1.1.14 — Clippy net_bind needless_return fix

## Correção

Corrigido erro de build/clippy em `src-tauri/src/net_bind.rs`:

```text
error: unneeded `return` statement
```

O método `bind_fixed_service_listener()` foi ajustado para retornar expressões diretamente nos braços do `match`, mantendo o mesmo comportamento de bind LAN e fallback de interface na mesma porta.

## Arquivo alterado

- `src-tauri/src/net_bind.rs`

## Impacto

- Não altera portas.
- Não altera splash screen.
- Não altera Webport/Webhook/WebSocket.
- Não altera regras de bind local/rede.
- Corrige apenas a falha `clippy::needless_return` apontada no build Windows.
