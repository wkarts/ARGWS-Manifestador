# Manifestador NF-e 1.1.14 — Visual Core v3, remoção da splash HTML e refinamento profissional

## Objetivo

Aplicar uma correção evolutiva sobre a remodelagem visual anterior, removendo a primeira splash screen HTML com progresso artificial e refinando a interface para ficar mais próxima do visual aprovado em mockup: mais limpa, compacta, profissional e com menor redundância visual.

## Alterações aplicadas

- Removida a splash HTML inicial do `index.html` (`manifestador-nfe-boot-fallback`).
- Reduzido o tempo mínimo da splash Vue realtime para evitar espera visual desnecessária.
- Mantido o fluxo real de inicialização via `runApplicationStartup`, sem alterar regras de negócio.
- Removida a faixa redundante de contexto da página abaixo das abas.
- Adicionado relógio discreto na topbar.
- Mantida empresa ativa no topo como ponto único de contexto empresarial.
- Removida duplicidade do status da API Interna no rodapé da sidebar.
- Mantido usuário apenas no menu superior com preferências de aparência, configurações e logout.
- Adicionados ícones visuais por rota na sidebar, sem dependência externa.
- Refinado o dashboard com cabeçalho compacto, cards mais modernos, atalhos com ícones e melhor densidade visual.
- Refinadas tabelas, filtros, cards, botões, switches, inputs, textareas e modais por CSS centralizado.
- Preservado o núcleo de temas por usuário em `src/visual-core/theme.ts`.

## Arquivos principais alterados

- `index.html`
- `src/App.vue`
- `src/layouts/AppLayout.vue`
- `src/pages/DashboardPage.vue`
- `src/components/dashboard/DashboardQuickActions.vue`
- `src/styles.css`

## Preservado

Não foram alterados algoritmos, services, stores sensíveis, endpoints, payloads, autenticação, permissões, sessão, licenciamento, API interna, Tauri/Rust ou persistência atual.

## Validação

Executado com sucesso:

```bash
npm run build:web:tauri
```

O build recompilou `dist` e `src-tauri/dist`.
