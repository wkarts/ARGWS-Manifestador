# 1.1.14 — Runtime Network Harmony + UI Fix

## Objetivo

Corrigir a instabilidade do startup quando os serviços são configurados com bind público (`0.0.0.0`) e reorganizar a tela **Sistema e parâmetros** para que o operador não precise digitar manualmente `127.0.0.1` ou `0.0.0.0`.

## Correções técnicas

### 1. Serviços não bloqueiam mais a janela Tauri

A inicialização da API interna, Webport/Proxy, Webhook Service e WebSocket Service foi movida para um orquestrador em thread própria. A janela Tauri passa a abrir primeiro, e os serviços sobem em segundo plano.

Arquivo principal:

- `src-tauri/src/lib.rs`

### 2. Splash preservado sem risco de tela branca

O `router-view` permanece montado desde o primeiro frame, atrás do splash. O splash continua existindo como tela de inicialização, porém não impede que a interface apareça caso algum serviço de rede falhe ou demore.

Arquivos:

- `src/App.vue`
- `src/services/startup.ts`

### 3. Webport usa configuração persistida, não variável antiga do processo

O `web_server` deixou de reconsultar diretamente `std::env` para decidir o host de bind. A fonte de verdade passa a ser `RuntimeSettings`, que já considera banco local e `.env`. Isso evita o caso em que o usuário salva LAN pela interface, mas uma variável antiga do processo mantém o Webport preso em `127.0.0.1`.

Arquivo:

- `src-tauri/src/internal_api/web_server.rs`

### 4. Salvamento de runtime reaplica ambiente e reinicia serviços controladamente

Ao salvar configurações, o runtime agora:

- normaliza os hosts auxiliares;
- atualiza variáveis de ambiente do processo;
- reinicia API/Webport/Webhook/WebSocket somente quando aplicável;
- preserva portas fixas configuradas pelo operador.

Arquivos:

- `src-tauri/src/runtime_config.rs`
- `src-tauri/src/commands/runtime.rs`

### 5. Tela Sistema e parâmetros reorganizada

A tela foi separada em cards por serviço:

- API interna;
- Webport/Proxy;
- Webhook Service;
- WebSocket Service;
- API/Scalar/segurança;
- Tray/startup;
- diretórios e execução.

Cada serviço agora possui botões de nível de acesso:

- **Local** → usa `127.0.0.1`;
- **Rede local/remoto** → usa `0.0.0.0` como bind.

O usuário não precisa digitar manualmente os hosts técnicos.

Arquivo:

- `src/pages/SystemPage.vue`

## Política de portas

As portas permanecem fixas e configuráveis:

- API interna: `61001`;
- Webport/Proxy: `61002`;
- Webhook Service: `61003`;
- WebSocket Service: `61004`.

Não há fallback automático para outra porta. Se a porta estiver ocupada por processo externo, o desktop segue abrindo e a tela mostra o estado sem travar.

## Observação

`0.0.0.0` é endereço de bind/escuta, não URL de consumo. A interface exibe URL local com `127.0.0.1` automaticamente, mesmo quando o bind está em rede.
