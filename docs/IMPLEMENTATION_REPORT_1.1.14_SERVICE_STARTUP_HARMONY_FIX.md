# Implementation Report 1.1.14 — Service Startup Harmony Fix

## Objetivo

Corrigir o comportamento onde o Webport/Proxy podia detectar a própria aplicação ou outra instância em aquecimento como conflito de porta, gerando `os error 10048` e mensagens repetidas de porta ocupada.

## Problema identificado

O startup estava descentralizado:

- `lib.rs` iniciava API interna.
- `internal_api::server::start_background` também tentava iniciar o Webport.
- `lib.rs` tentava iniciar o Webport novamente.
- O frontend/splash também chamava `web_proxy_start`.

Com isso, a mesma porta podia receber múltiplas tentativas de bind em sequência. Em Windows, quando a primeira rotina já abriu o socket, qualquer segunda tentativa na mesma porta retorna `WSAEADDRINUSE / os error 10048`, mesmo que o dono da porta seja a própria aplicação ou uma instância ainda inicializando.

## Correções aplicadas

### 1. Orquestração única no startup

A API interna não inicia mais o Webport internamente. O startup dos serviços fica centralizado no fluxo principal do Tauri/CLI.

Arquivo alterado:

- `src-tauri/src/internal_api/server.rs`

### 2. Webport idempotente com adoção tardia

Ao receber `AddrInUse`, o Webport não conclui imediatamente que a porta pertence a outro processo. Ele aguarda alguns ciclos e tenta reconhecer uma instância válida do próprio Manifestador NF-e por:

- `http://127.0.0.1:<porta>/__web_health`
- `http://127.0.0.1:<porta>/`

Isso evita falso positivo quando a primeira instância ainda está aquecendo.

Arquivo alterado:

- `src-tauri/src/internal_api/web_server.rs`

### 3. Splash não inicia porta novamente

O splash/frontend não chama mais `web_proxy_start`. Ele apenas consulta `web_proxy_status`. O serviço deve ser iniciado pelo runtime nativo, não pela interface visual.

Arquivo alterado:

- `src/services/startup.ts`

### 4. Locks de inicialização para serviços auxiliares

Adicionados locks de inicialização para evitar chamadas concorrentes de start do Webhook e do WebSocket.

Arquivos alterados:

- `src-tauri/src/native_webhook/server.rs`
- `src-tauri/src/native_websocket/server.rs`

### 5. Portas fixas preservadas

O Webport, Webhook e WebSocket continuam respeitando as portas configuradas. Não há auto-port para `61003`, `61004` etc. O parâmetro configurado é a fonte de verdade.

## Validação realizada

Executado com sucesso:

```bash
npm run typecheck
npm run build:web
node ./scripts/prepare-tauri-resources.mjs
```

Não foi possível executar `cargo check` neste ambiente porque `cargo/rustc` não está instalado.

## Resultado esperado

- A primeira instância abre os serviços habilitados nas portas configuradas.
- A segunda instância não quebra a interface por conflito de porta.
- A aplicação não tenta iniciar Webport várias vezes durante o splash.
- `61002` permanece fixa para Webport.
- `61003` permanece fixa para Webhook.
- `61004` permanece fixa para WebSocket.
- Se uma porta estiver ocupada por instância válida do próprio app, ela é reutilizada.
- Se uma porta estiver ocupada por processo externo, a interface desktop continua funcionando e o log informa o motivo.
