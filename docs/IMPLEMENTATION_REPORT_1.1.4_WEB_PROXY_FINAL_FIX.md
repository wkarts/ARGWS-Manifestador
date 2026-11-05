# Correção final 1.1.4 — Servidor Web local, proxy interno e acesso LAN

## Objetivo

Corrigir definitivamente o problema onde a API interna em `61001` funcionava, mas a aplicação Web em `61002` não podia ser acessada pelo navegador, principalmente via rede local.

## Correções aplicadas

- Adicionado servidor Web local Rust/Tauri na porta configurada por `MANIFESTADOR_NFE_WEB_PORT`.
- O servidor Web publica a pasta `dist` gerada pelo Vite com fallback SPA para `index.html`.
- O servidor Web sobe automaticamente por padrão quando o app Tauri inicia.
- O bind Web agora usa `0.0.0.0` por padrão para permitir acesso em localhost e rede local.
- Mesmo que configurações antigas tenham gravado `127.0.0.1`, o servidor pode forçar bind LAN via `MANIFESTADOR_NFE_WEB_BIND_LAN=true`.
- Adicionado proxy real em `/__internal_api/*` dentro da porta Web.
- O proxy encaminha requisições para a API interna configurada, normalmente `http://127.0.0.1:61001`.
- O proxy aplica automaticamente o token da API interna quando configurado.
- Adicionada publicação dos assets Web:
  - `/assets/*`
  - `/branding/*`
  - `/icons/*`
  - `/manifest.webmanifest`
  - `/sw.js`
- Adicionado `../dist/**/*` como recurso de bundle Tauri para a aplicação instalada conseguir localizar os arquivos Web quando necessário.

## URLs esperadas

Com a aplicação Tauri aberta:

- `http://127.0.0.1:61002/`
- `http://IP_DA_MAQUINA:61002/`
- `http://127.0.0.1:61002/__internal_api/health`
- `http://IP_DA_MAQUINA:61002/__internal_api/health`
- `http://127.0.0.1:61002/__internal_api/docs`
- `http://IP_DA_MAQUINA:61002/__internal_api/docs`

## Variáveis adicionadas

```env
MANIFESTADOR_NFE_WEB_HOST=0.0.0.0
MANIFESTADOR_NFE_WEB_PORT=61002
MANIFESTADOR_NFE_WEB_AUTO_START=true
MANIFESTADOR_NFE_WEB_BIND_LAN=true
MANIFESTADOR_NFE_WEB_DIST_DIR=
```

## Observação

`0.0.0.0` deve ser usado apenas para bind/listen. Para navegação no browser, use `127.0.0.1` ou o IP real da máquina.
