# Manifestador NF-e Tauri 1.1.14 — Webport in-process command bridge e otimização

## Objetivo

Corrigir os avisos exibidos no navegador pelo webport:

- `Falha ao interpretar resposta da API interna para app_bootstrap.`
- `Falha ao interpretar resposta da API interna para web_proxy_start.`

E reduzir a lentidão percebida quando a aplicação é aberta via navegador em `http://127.0.0.1:61002`.

## Causa encontrada

A página web já estava sendo servida pelo processo Tauri na porta `61002`, porém os comandos do frontend eram enviados para:

```text
/__internal_api/api/command
```

Esse endpoint passava por um proxy HTTP interno e dependia de `internal_api_base_url`. Em ambientes com configuração local antiga ou inconsistente, essa base podia apontar para a própria porta web `61002`, para um caminho com `/__internal_api` ou para uma API interna ainda indisponível.

Quando isso acontecia, o navegador recebia HTML/texto no lugar do JSON esperado pelo `InternalApiCommandProvider`, causando erro de interpretação.

Além disso, cada chamada proxy criava um novo `reqwest::Client`, deixando o modo navegador mais lento que o modo desktop.

## Correção aplicada

### 1. Command bridge direto dentro do processo webport

O endpoint:

```text
POST /__internal_api/api/command
```

agora executa diretamente os comandos via `app_commands::execute_command`, usando `InternalApiState` dentro do próprio servidor webport.

Isso elimina para os comandos principais:

- hop HTTP extra;
- dependência de CORS;
- dependência de token externo;
- dependência de `internal_api_base_url` para comandos da interface;
- risco de receber HTML em vez de JSON.

### 2. Health direto no webport

O endpoint:

```text
GET /__internal_api/health
```

agora responde direto pelo webport, confirmando que a ponte in-process está ativa.

### 3. Proxy HTTP mantido apenas como fallback

O proxy `/__internal_api/*` continua existindo para rotas auxiliares, mas agora:

- reutiliza um `reqwest::Client` persistente;
- evita base URL apontando para a própria porta web;
- remove sufixo `/__internal_api` de configuração persistida antiga.

### 4. Startup web sem chamada redundante

No navegador, a aplicação não chama mais `web_proxy_start`, porque o webport já está necessariamente ativo para a página ter carregado.

No Tauri desktop, a chamada continua válida como reforço nativo via `invoke`, sem CMD/PowerShell.

### 5. Cache de assets estáticos

Assets versionados em `/assets/*` agora usam:

```text
Cache-Control: public, max-age=31536000, immutable
```

O `index.html` continua sem cache para refletir atualizações corretamente.

## Arquivos alterados

```text
src-tauri/src/internal_api/web_server.rs
src-tauri/src/internal_api/web_app.rs
src-tauri/src/internal_api/app_commands.rs
src/services/startup.ts
src/core/invoker/InternalApiCommandProvider.ts
dist/assets/index-*.js
src-tauri/dist/assets/index-*.js
```

## Resultado esperado

Com o aplicativo Tauri aberto normalmente:

```text
http://127.0.0.1:61002
```

A tela web deve carregar sem os avisos de `app_bootstrap` e `web_proxy_start`.

O teste:

```text
http://127.0.0.1:61002/__internal_api/health
```

Deve retornar JSON com:

```json
{
  "ok": true,
  "service": "internal-api-via-webport",
  "transport": "in-process"
}
```

## Observação

A correção não adiciona CMD, PowerShell, `.bat`, script manual ou processo externo. O webport permanece interno ao processo Tauri.
