# Implementação 1.1.14 — Visual Core v5: Sidebar estreita, menu do usuário, temas e dashboard

## Objetivo
Aplicar correções visuais sem alterar regra de negócio, algoritmos, services, stores sensíveis, endpoints, payloads, autenticação, permissões, sessão, licenciamento, API interna, Tauri/Rust ou persistência atual.

## Correções aplicadas

- O botão sandwich agora recolhe a sidebar para uma barra estreita de ícones, sem esconder toda a navegação no desktop.
- Em telas pequenas, o comportamento segue como drawer responsivo para não prejudicar mobile/tablet.
- O menu do usuário agora fica sempre posicionado logo abaixo do botão do usuário na topbar, independente de usar guias/abas ou não.
- Foi criada uma área própria para gerenciar temas do usuário, com criação, edição e exclusão de até 5 temas próprios.
- Os temas predefinidos atuais foram mantidos.
- Foram adicionados temas com background/sidebar colorida, preservando a linha visual anterior.
- O dashboard foi refinado para ficar mais próximo da proposta aprovada, com cartões mais profissionais, melhor composição visual, eventos recentes e prévias visuais de gráficos.
- O gerenciamento de tema continua isolado por usuário via localStorage, sem impactar outros usuários.

## Arquivos principais alterados

- `src/layouts/AppLayout.vue`
- `src/pages/DashboardPage.vue`
- `src/visual-core/theme.ts`
- `src/styles.css`
- `dist/*`
- `src-tauri/dist/*`

## Validação

```bash
npm run build:web:tauri
```

Resultado: build concluído com sucesso.

## Observações

A implementação permanece visual/herdável. Não foram alterados fluxos de negócio nem contratos internos dos serviços existentes.
