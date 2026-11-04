# Manifestador NF-e Tauri Desktop Web 1.1.5 — Correção final Web Proxy e Serviços

## Objetivo

Corrigir os problemas reportados na ativação dos serviços nativos e no proxy web da porta 61002.

## Correções aplicadas

- O servidor Web/proxy não usa mais 61003 como fallback.
- A porta 61003 permanece reservada exclusivamente para Webhook Service.
- A porta 61004 permanece reservada exclusivamente para WebSocket Service.
- Para evitar travamento quando a porta 61002 estiver ocupada por processo antigo ou outro aplicativo, foi adicionado auto-port seguro para o Web/proxy no intervalo 61020-61099.
- O auto-port do Web/proxy é configurável por `MANIFESTADOR_NFE_WEB_AUTO_PORT` ou `APP_WEB_AUTO_PORT`.
- URLs exibidas nas telas de Webhook e WebSocket agora convertem `0.0.0.0` para `127.0.0.1`, evitando links inválidos para cliente.
- A tela Sistema não exibe mais `external-command-disabled`; serviço nativo externo passa a aparecer como gerenciado por instalador/helper nativo.
- Botões de iniciar/parar/reiniciar serviço do sistema foram desabilitados na UI para evitar falsa expectativa, pois a aplicação não executa cmd/PowerShell/sc/systemctl em runtime.

## Portas

- 61001: API interna REST/Scalar.
- 61002: Web/proxy configurado.
- 61020-61099: range alternativo seguro apenas para Web/proxy quando `MANIFESTADOR_NFE_WEB_AUTO_PORT=true`.
- 61003: Webhook Service.
- 61004: WebSocket Service.

## Observação

Se o Web/proxy subir em porta alternativa, a tela de parâmetros e os logs devem exibir a porta efetiva. Isso evita colisão com Webhook/WebSocket e mantém a aplicação acessível pelo navegador.
