# Implementação 1.1.14 — Visual Core v8: Topbar, modais, responsividade e TitleBar padrão

## Objetivo
Corrigir os pontos visuais apontados após a validação da versão v7, mantendo a lógica da aplicação intacta.

## Correções aplicadas

- Topbar não fica mais acima dos modais.
- Overlay e card dos modais agora possuem prioridade visual superior ao shell, topbar e guias.
- Relógio/data da topbar foi reorganizado para exibir:
  - dia da semana e mês;
  - horário e data numérica;
  - tooltip com data/hora completa.
- Campo de busca foi reduzido para melhorar proporção da topbar.
- Select de empresa ativa foi ajustado para ficar mais proporcional.
- Topbar recebeu regras responsivas para não quebrar em telas menores.
- Em telas menores, a sidebar não fica simultaneamente em modo estreito/expandido.
- Em telas menores, a sidebar fica oculta e abre como drawer somente ao clicar no botão sandwich.
- BasePage foi ajustado para usar o mesmo padrão visual de TitleBar da dashboard.
- Páginas antigas que usam toolbar no topo também recebem o mesmo padrão visual por CSS herdável.

## Arquivos principais alterados

- `src/layouts/AppLayout.vue`
- `src/components/base/BasePage.vue`
- `src/config/projectConfig.ts`
- `src/styles.css`

## Validação

Comando executado com sucesso:

```bash
npm run build:web:tauri
```

## Preservado

- Regras de negócio
- Algoritmos
- Services sensíveis
- Stores sensíveis
- Payloads
- Endpoints
- Autenticação
- Permissões
- Sessão
- Licenciamento
- API interna
- Tauri/Rust
- Persistência atual
