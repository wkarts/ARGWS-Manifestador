# Visual Core v13.1 — Ajuste fino da Dashboard

## Objetivo

Corrigir apenas o comportamento visual da Dashboard quando o usuário abre a personalização e remove/oculta widgets, evitando que a titlebar e a barra de parâmetros fiquem com altura desproporcional.

## Alterações

- Redução e limitação controlada da altura da `dashboard-command-center`.
- Compactação do painel `dashboard-customizer-panel`.
- Ajuste de espaçamento, peso de fonte e altura dos itens de seleção de widgets.
- Preservada responsividade para telas menores.
- Patch aplicado também nos arquivos já compilados de `dist` e `src-tauri/dist`.

## Escopo preservado

- Sem alteração de regras de negócio.
- Sem alteração de stores.
- Sem alteração de services.
- Sem alteração de Tauri/Rust.
- Sem alteração de endpoints ou payloads.
- Sem refatoração estrutural.
