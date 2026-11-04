# Correção 1.1.5 — Conflito de portas entre Web, Webhook e WebSocket

Esta correção removeu o fallback automático de portas entre serviços.

## Problema identificado

Quando a porta Web `61002` estava ocupada, o servidor Web tentava usar `61003` como fallback. A porta `61003` passou a ser reservada para o Webhook Service, causando conflito quando os serviços eram ativados.

Também havia fallback semelhante em API interna, Webhook e WebSocket. Isso poderia fazer um serviço invadir a porta reservada de outro serviço.

## Correção aplicada

- API interna não faz mais fallback automático de porta.
- Web/proxy não faz mais fallback automático de porta.
- Webhook Service não faz mais fallback automático de porta.
- WebSocket Service não faz mais fallback automático de porta.
- `build_ports` mantém `effective_port = configured_port`.
- A tela/diagnóstico passa a reportar porta ocupada como erro de configuração, sem alterar a porta automaticamente.

## Mapa de portas preservado

- `61001`: API interna REST/Scalar.
- `61002`: Web app/proxy.
- `61003`: Webhook Service.
- `61004`: WebSocket Service.

## Comportamento esperado

Se uma porta estiver ocupada, o serviço falha com mensagem clara e a aplicação permanece aberta. A correção deve ser feita alterando a porta no `.env`/parâmetros ou encerrando o processo que está usando a porta.

