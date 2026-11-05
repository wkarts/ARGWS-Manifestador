# Manifestador NF-e Tauri 1.1.14 — Desktop Blank Screen + Webport Recovery Fix

## Objetivo

Corrigir a regressão onde a janela Tauri podia ficar em branco quando o webport/API encontrava porta ocupada por múltiplas instâncias, preservando a recuperação de multi-instância e mantendo o webport preparado para LAN quando a primeira instância consegue publicar a porta.

## Causa técnica

O erro `os error 10048` indica porta já ocupada no Windows. Como o webport estava em modo estrito para LAN (`0.0.0.0:61002`), uma segunda instância não conseguia bindar a mesma porta. A recuperação de instância existente era boa, mas quando a porta estava ocupada por processo antigo ou por versão que não respondia ao health endpoint, o startup ainda registrava falha e a experiência podia ficar com tela branca ou sem fallback visual.

## Correções aplicadas

- O router da aplicação agora é montado desde o primeiro frame; o splash virou apenas overlay visual.
- A tela Tauri não fica mais dependente do sucesso do startup para exibir a interface.
- O tempo máximo de splash seguro foi reduzido para liberar a interface mais rapidamente.
- O `index.html` agora possui fallback HTML/CSS nativo antes do Vue carregar, evitando janela totalmente branca caso o JavaScript falhe.
- O `main.ts` renderiza fallback visível se o bootstrap Vue falhar.
- O webport agora tenta reconhecer instâncias existentes por `/__web_health` e também por `/`, cobrindo versões anteriores que serviam o app mas não tinham o endpoint de health.
- Se a porta 61002 estiver ocupada por processo externo/não reconhecido, o erro é tratado como não fatal para o desktop. A aplicação continua abrindo normalmente e o webport fica indisponível até a porta ser liberada ou alterada.

## Arquivos principais alterados

- `index.html`
- `src/main.ts`
- `src/App.vue`
- `src/services/startup.ts`
- `src-tauri/src/internal_api/web_server.rs`
- `dist/`
- `src-tauri/dist/`

## Validação realizada

- `npm run typecheck`
- `npm run build:web`
- `node ./scripts/prepare-tauri-resources.mjs`

O build Rust não foi validado neste ambiente porque `cargo` não está instalado no sandbox.

## Comportamento esperado

- Abrir várias instâncias não deve deixar a janela em branco.
- Se a porta 61002 já estiver ocupada por uma instância válida, ela será reutilizada.
- Se a porta 61002 estiver ocupada por processo externo, a aplicação desktop continua exibindo a interface normalmente.
- A primeira instância válida ainda tenta publicar o webport em `0.0.0.0:61002` para acesso LAN/remoto.
