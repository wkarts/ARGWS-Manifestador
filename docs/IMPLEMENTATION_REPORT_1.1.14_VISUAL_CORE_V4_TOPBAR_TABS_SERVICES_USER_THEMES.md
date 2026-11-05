# Manifestador NF-e 1.1.14 — Visual Core v4

## Objetivo

Aplicar correções finas sobre o Visual Core v3 para aproximar a aplicação do mockup aprovado, mantendo a regra de negócio, stores, services sensíveis, endpoints, payloads, autenticação, permissões, licenciamento, API interna e runtime Tauri/Rust preservados.

## Ajustes aplicados

- Corrigido comportamento do menu do usuário na topbar com guias/abas e sem guias/abas.
- Menu do usuário agora usa posição fixa calculada para não ficar desalinhado quando a barra de guias está ativa.
- Sidebar agora recolhe corretamente ao clicar no botão sandwich.
- Busca global da topbar agora aceita foco, texto, atalho Ctrl+K e navega para páginas/módulos encontrados.
- Removidos símbolos estranhos e substituídos por ícones SVG internos consistentes.
- Ícones do menu lateral revisados para concordância com cada rota.
- Ícones da topbar revisados: atualizar, tema, notificações, menu, sair e configurações.
- Dashboard e atalhos rápidos passaram a usar ícones SVG, não emojis/símbolos soltos.
- Status de serviços na topbar foi alterado para três indicadores independentes: API, Webhook e WebSocket.
- Cada serviço exibe LED visual e tooltip/hint com endereço/porta/status, sem texto Online/Offline abaixo do nome.
- Guias/abas foram remodeladas para estilo compacto tipo workspace, com aba ativa em destaque azul.
- Tema padrão alterado para claro com sidebar clara.
- Mantidos presets de temas predefinidos.
- Adicionada criação de até 5 temas próprios por usuário via preferências visuais locais.
- Preferências visuais continuam isoladas por usuário logado.

## Arquivos principais alterados

- `src/layouts/AppLayout.vue`
- `src/visual-core/theme.ts`
- `src/styles.css`
- `src/components/base/IconSymbol.vue`
- `src/components/dashboard/DashboardMetricCard.vue`
- `src/components/dashboard/DashboardQuickActions.vue`
- `src/pages/DashboardPage.vue`

## Validação

Comando executado com sucesso:

```bash
npm run build:web:tauri
```

O build web foi gerado e copiado para `src-tauri/dist`.
