# Implementação 1.1.6 — Paridade School Web Proxy

## Objetivo

Aplicar no template o mesmo padrão funcional observado no `school-desktop-suite-template-web-1.1.3-etapa46`, garantindo que a aplicação Tauri também seja publicada no navegador pela porta Web configurada, por padrão `61002`.

## Correções aplicadas

- Servidor Web interno do Tauri mantém a porta configurada `61002` como porta principal.
- Removido auto fallback de porta Web para range alternativo como comportamento padrão.
- Se a porta `61002` já estiver ocupada por um servidor HTTP funcional, o template reaproveita o servidor existente e retorna status sem quebrar a interface.
- Se a porta estiver ocupada por processo inválido, o serviço retorna erro claro, sem invadir portas reservadas de Webhook ou WebSocket.
- O Web server continua servindo a mesma aplicação do Tauri e o proxy `/__internal_api/*` encaminha para a API interna.
- Build Web configurado com `minify: false`, `cssMinify: false`, `sourcemap: true` e `base: "./"`.
- Removido uso de assets externos na documentação da API: o HTML de documentação agora é local, sem CDN.
- Ajustado empacotamento para copiar `dist` para `src-tauri/dist`, evitando `_up_/dist`.
- Bundle Tauri agora usa resource limpo `dist/**/*`.

## Portas

- `61001`: API interna REST/OpenAPI.
- `61002`: Web App + proxy `/__internal_api/*`.
- `61003`: Webhook Service.
- `61004`: WebSocket Service.

## Testes recomendados

Após abrir o app Tauri:

```text
http://127.0.0.1:61002/
http://127.0.0.1:61002/__internal_api/health
http://127.0.0.1:61002/__internal_api/docs
```

Em rede local:

```text
http://IP_DA_MAQUINA:61002/
http://IP_DA_MAQUINA:61002/__internal_api/health
```

## Observação

A porta `61002` precisa estar livre ou ocupada por uma instância saudável do próprio servidor Web do template. O template não mata processos externos em runtime.
