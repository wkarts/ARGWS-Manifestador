# Implementação 1.1.8 — Correção definitiva do Web Proxy e WebSocket

## Objetivo

Corrigir o template para manter o comportamento solicitado:

- a aplicação Tauri continua funcionando na janela desktop;
- a mesma interface também é publicada no navegador pela porta Web configurada, por padrão `61002`;
- o proxy `/__internal_api/*` continua encaminhando para a API interna em `61001`;
- a documentação Scalar padrão foi preservada;
- o WebSocket Service não quebra a interface quando a porta padrão está ocupada por instância anterior.

## Correções

### Web/proxy

O servidor Web/proxy passa a ser iniciado como recurso estrutural do template desktop-web, salvo quando explicitamente desabilitado por:

```env
MANIFESTADOR_NFE_WEB_DISABLED=true
```

Também foi adicionada chamada idempotente no `app_bootstrap`, garantindo que o servidor Web/proxy seja tentado novamente quando o frontend inicializa.

### Webhook/WebSocket

Os serviços agora têm ranges alternativos seguros quando a porta padrão estiver ocupada:

- Webhook padrão: `61003`
- Webhook fallback seguro: `61030-61039`
- WebSocket padrão: `61004`
- WebSocket fallback seguro: `61040-61079`

Esses fallbacks não invadem portas reservadas da API interna ou Web/proxy.

### Scalar

A documentação Scalar padrão com `@scalar/api-reference` foi preservada. A correção não substitui Scalar por tela customizada.

## Testes recomendados no Windows

```bat
build-windows.bat -Clean -InstallMode Install
```

Depois de abrir o app Tauri:

```text
http://127.0.0.1:61002/
http://127.0.0.1:61002/__internal_api/health
http://127.0.0.1:61002/__internal_api/docs
http://127.0.0.1:61001/docs
```

WebSocket:

```text
ws://127.0.0.1:61004/ws
```

Se a porta `61004` estiver ocupada, a tela administrativa deve informar a porta alternativa usada no range `61040-61079`.
