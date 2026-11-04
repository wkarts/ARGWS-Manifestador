# Implementação 1.1.14 — Web/proxy no padrão School

Esta versão corrige o ciclo de inicialização do Web/proxy do template.

## Problema

O Web/proxy estava acoplado ao ciclo da API interna. Quando a porta da API (`61001`) estava ocupada por instância anterior, a API falhava antes do Web/proxy ser iniciado. O resultado era:

- app Tauri abria parcialmente;
- navegador em `http://127.0.0.1:61002` recusava conexão;
- logs mostravam apenas falha da API, sem servidor Web ativo.

## Correção

- `internal_api::web_server::start_background` foi reescrito no padrão do School, com `StdTcpListener::bind` antes da thread Tokio.
- O Web/proxy inicia independentemente da API interna durante o setup Tauri.
- A porta padrão do Web/proxy passa a ser `127.0.0.1:61002` por padrão.
- O proxy `/__internal_api/*` continua encaminhando para a API interna `61001`.
- Se a API estiver indisponível, a interface Web continua abrindo; apenas chamadas ao proxy retornam erro HTTP.
- Se `61002` estiver realmente ocupada por outro processo, o erro fica claro no log.

## Testes esperados

Após abrir o Tauri:

- `http://127.0.0.1:61002/`
- `http://127.0.0.1:61002/__internal_api/health`
- `http://127.0.0.1:61002/__internal_api/docs`

