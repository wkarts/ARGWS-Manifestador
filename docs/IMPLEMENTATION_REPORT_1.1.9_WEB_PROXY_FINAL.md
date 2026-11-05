# Implementação 1.1.9 — Correção definitiva do Web/proxy 61002

## Objetivo

Fazer o template publicar, como o School Desktop Suite, a mesma interface interna do Tauri no navegador pela porta Web configurada, por padrão `61002`, sem depender de minificação, sem assets externos e sem usar porta salva antiga no banco local.

## Problema corrigido

A porta Web podia não subir em `61002` porque o servidor Web/proxy carregava configurações persistidas em banco/local settings. Em ambientes que passaram por várias versões, isso podia deixar a porta Web em valor obsoleto ou impedir o listener correto em `127.0.0.1:61002`.

## Correção aplicada

- A API interna agora inicia por padrão no bootstrap nativo, salvo desativação explícita por env.
- O Web/proxy agora inicia por padrão no bootstrap nativo, salvo desativação explícita por env.
- A porta e o host do Web/proxy não dependem mais da configuração persistida antiga.
- O Web/proxy usa apenas:
  - `MANIFESTADOR_NFE_WEB_HOST` ou `APP_WEB_HOST`
  - `MANIFESTADOR_NFE_WEB_PORT` ou `APP_WEB_PORT`
  - padrão: `0.0.0.0:61002`
- O proxy `/__internal_api/*` continua apontando para a API interna em `61001`.

## Testes esperados

Com o Tauri aberto:

- `http://127.0.0.1:61002/`
- `http://127.0.0.1:61002/__internal_api/health`
- `http://127.0.0.1:61002/__internal_api/docs`

Na rede local:

- `http://IP_DA_MAQUINA:61002/`
- `http://IP_DA_MAQUINA:61002/__internal_api/health`

## Observação

Se `61002` continuar recusando conexão, existe alta chance de o executável aberto não ser desta versão, de haver `.env.local` desativando o Web/proxy, ou de o app não ter sido reiniciado após a troca do build.
