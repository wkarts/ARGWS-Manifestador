# Manifestador NF-e 1.1.14 - Runtime Startup Stability Fix

Correção sem refatoração abrupta para harmonizar startup dos serviços internos e preservar a tela.

## Ajustes

- A configuração salva pela interface passa a ser a fonte de verdade para iniciar Webport, Webhook e WebSocket.
- Variáveis de ambiente `*_ENABLED=false` antigas não bloqueiam mais um serviço salvo como habilitado pela interface; apenas `*_DISABLED=true` é tratado como bloqueio explícito.
- Corrigida duplicidade/sintaxe no menu de tray ao iniciar API interna.
- Webport não consulta mais flag antiga `MANIFESTADOR_NFE_WEB_BIND_LAN` para decidir bind quando a tela já salvou `local_web_host`.
- Splash screen permanece visível por tempo mínimo maior, sem bloquear a interface.
- Página Sistema e parâmetros recebeu correção de rolagem para exibir todo o conteúdo dentro do workspace.

## Política de portas

- API interna: porta fixa configurada.
- Webport/Proxy: porta fixa configurada.
- Webhook Service: porta fixa configurada.
- WebSocket Service: porta fixa configurada.
- Não há auto-port para Webport.
- 0.0.0.0 é usado apenas como endereço de bind; URLs locais exibidas usam 127.0.0.1.
