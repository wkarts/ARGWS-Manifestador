
## Patch complementar: Clippy Windows

Corrigidos warnings tratados como erro pelo `cargo clippy -- -D warnings` no Windows:

- removido `mut` desnecessário em `native_webhook::server::start_background_with_config`;
- removido `mut` desnecessário em `native_websocket::server::start_background_with_config`;
- removida função `find_available_port`, que ficou sem uso após a remoção definitiva do fallback automático de portas.

Essas correções preservam a regra principal desta versão: nenhum serviço deve procurar porta alternativa automaticamente, evitando conflito entre API interna, Web/proxy, Webhook e WebSocket.
