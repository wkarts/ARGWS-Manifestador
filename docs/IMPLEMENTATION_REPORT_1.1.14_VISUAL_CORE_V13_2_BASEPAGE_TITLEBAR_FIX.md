# Manifestador NF-e 1.1.14 — Visual Core v13.2 BasePage TitleBar Fix

## Objetivo

Aplicar ajuste pontual para que a `BasePage` use o mesmo comportamento visual da titlebar da Dashboard, preservando a harmonia do tema e sem refatoração abrupta.

## Alterações

- Ajuste fino em `src/styles.css`.
- `.base-page > .page-header-card.standard-page-titlebar` passa a usar a mesma proporção visual da `.dashboard-page .dashboard-command-center`.
- Corrigidos altura, padding, borda, radius, gradiente, sombra, ícone, título, subtítulo e ações da titlebar das páginas BasePage.
- Mantida a Dashboard intocada.

## Preservado

- Regras de negócio.
- Services.
- Stores.
- Endpoints.
- Payloads.
- API interna.
- Tauri/Rust.
- Mecânica de páginas, abas, modais e dashboard.

## Validação

Executado:

```bash
npm run build:web:tauri
```

