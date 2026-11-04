# Manifestador NF-e Tauri 1.1.14 — Correção webport LAN/remoto

## Tipo de entrega

Patch corretivo incremental sobre a versão 1.1.14 com webport in-process.

## Problema

Após a correção do webport para iniciar automaticamente pelo processo Tauri, o servidor passou a responder apenas em `127.0.0.1:61002`.

Isso acontecia porque versões anteriores gravavam `MANIFESTADOR_NFE_WEB_HOST=127.0.0.1` no `.env` e/ou `local_web_host=127.0.0.1` em `runtime.settings`. Com esse bind, o socket aceita somente conexões locais da própria máquina. Outros computadores da rede recebem conexão recusada.

## Correção aplicada

O webport agora usa bind padrão em todas as interfaces:

```env
MANIFESTADOR_NFE_WEB_HOST=0.0.0.0
MANIFESTADOR_NFE_WEB_PORT=61002
MANIFESTADOR_NFE_WEB_BIND_LAN=true
```

Também foi adicionada normalização defensiva no runtime:

- `127.0.0.1`, `localhost`, `::1`, `[::1]` e vazio são promovidos para `0.0.0.0` no webport.
- Configurações antigas persistidas pelo usuário deixam de prender o webport em localhost.
- O acesso local continua funcionando por `http://127.0.0.1:61002`.
- O acesso em rede passa a funcionar por `http://IP_DA_MAQUINA:61002`.

## Observações

A aplicação não executa CMD/PowerShell para liberar firewall ou criar regra de roteador. Ela apenas abre o socket corretamente em `0.0.0.0:61002` dentro do próprio processo Tauri.

Para acesso remoto fora da LAN, ainda é necessário que a infraestrutura permita entrada na porta 61002, por firewall, NAT, VPN, túnel reverso ou proxy externo.

## Arquivos alterados

- `.env.example`
- `.env.model`
- `src-tauri/src/runtime_config.rs`
- `src-tauri/src/internal_api/web_server.rs`
