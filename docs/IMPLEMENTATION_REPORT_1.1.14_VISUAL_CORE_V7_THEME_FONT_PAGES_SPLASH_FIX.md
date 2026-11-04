# Implementação 1.1.14 — Visual Core v7: temas, fontes, topbar, páginas e splash

## Objetivo
Aplicar refinamentos sobre o Visual Core v6 mantendo regras de negócio, services, payloads, endpoints, autenticação, sessão, licenciamento e integrações existentes.

## Alterações aplicadas

- Corrigido contraste da área de logo/branding em temas escuros e temas personalizados com sidebar escura.
- Adicionadas variáveis visuais para tipografia por tema personalizado:
  - família da fonte;
  - escala do texto;
  - escala dos títulos;
  - cor de texto;
  - cor de texto secundário.
- Mantidos temas padrão sem alteração física. Ao personalizar visual, o usuário salva como tema próprio.
- Removido limite de quantidade total de temas personalizados salvos.
- Mantido limite de até 5 temas personalizados ativos no menu rápido do usuário.
- Adicionada importação/exportação de temas em JSON para distribuição entre usuários.
- Melhorado select de empresa ativa na topbar.
- Melhorado relógio da topbar com largura mais adequada e tooltip contendo data/hora completa.
- Reduzida largura do campo de busca da topbar.
- Melhorado modal de perfil de usuário com agrupamento em cards e permissões em controles tipo switch/card.
- Melhorado modal de usuário com agrupamento por seções e seleção de perfis/empresas por cards selecionáveis.
- Melhorada organização visual geral da tela de licenciamento via padrões de cards, seções e switches.
- Reduzido tempo de exibição da splash realtime para evitar parada aparente em 96%.

## Validação
Executado com sucesso:

```bash
npm run build:web:tauri
```

## Arquivos principais alterados

- `src/visual-core/theme.ts`
- `src/layouts/AppLayout.vue`
- `src/styles.css`
- `src/pages/PerfilPage.vue`
- `src/pages/UsuarioPage.vue`
- `src/services/startup.ts`
- `src/App.vue`

## Pontos preservados

- Regras de negócio
- Algoritmos
- Services sensíveis
- Stores sensíveis
- Endpoints
- Payloads
- Autenticação
- Permissões
- Sessão
- Licenciamento
- API interna
- Tauri/Rust
- Persistência atual
