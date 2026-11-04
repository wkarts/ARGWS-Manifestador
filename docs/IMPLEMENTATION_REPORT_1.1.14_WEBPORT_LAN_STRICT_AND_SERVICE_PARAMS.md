# Patch 1.1.14 — Webport LAN estrito e parâmetros de autostart dos serviços

## Objetivo

Corrigir o cenário em que o Webport/Proxy abria em `localhost/127.0.0.1`, mas não ficava acessível pela rede local ou remoto, enquanto Webhook Service e WebSocket Service funcionavam normalmente.

## Causa corrigida

O Webport tinha fallback silencioso para `127.0.0.1` quando o bind em `0.0.0.0` falhava. Esse fallback deixava o navegador local funcionando, mas bloqueava completamente acesso por IP da rede.

Agora o Webport não faz fallback silencioso para localhost. Se estiver configurado para LAN, ele precisa subir em `0.0.0.0:61002`; se não conseguir, registra erro explícito para correção da porta/host.

## Parâmetros adicionados

O executável passa a aceitar flags para iniciar serviços automaticamente junto com o processo:

- `--start-web-proxy`
- `--start-webhook`
- `--start-websocket`
- `--start-services`
- aliases: `--webhook-auto-start`, `--websocket-auto-start`, `--services-auto-start`

Também foram adicionadas variáveis de ambiente:

- `MANIFESTADOR_NFE_WEBHOOK_AUTO_START=true`
- `MANIFESTADOR_NFE_WEBSOCKET_AUTO_START=true`
- `MANIFESTADOR_NFE_SERVICES_AUTO_START=true`

## Arquivos alterados

- `src-tauri/src/internal_api/web_server.rs`
- `src-tauri/src/cli/args.rs`
- `src-tauri/src/cli/runner.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/runtime_config.rs`
- `.env.example`
- `.env.model`

## Resultado esperado

Com o aplicativo aberto normalmente, o Webport deve escutar em `0.0.0.0:61002`, permitindo:

- `http://127.0.0.1:61002`
- `http://IP_DA_MAQUINA:61002`
- acesso remoto quando NAT/firewall/proxy reverso já estiverem configurados.
