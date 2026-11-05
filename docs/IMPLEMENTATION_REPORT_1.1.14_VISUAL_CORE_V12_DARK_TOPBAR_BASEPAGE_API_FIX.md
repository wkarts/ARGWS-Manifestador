# Implementation Report — 1.1.14 Visual Core v12

## Escopo
Correção visual e funcional incremental sobre a versão v11, mantendo regras de negócio e serviços sensíveis preservados.

## Ajustes aplicados

- Tema `Escuro azul` renomeado para `Azul escuro`.
- Modo escuro refeito para ser adaptativo ao tema selecionado, usando a base da sidebar/background do tema em vez do dark genérico acinzentado.
- Melhorado contraste de cards, titlebar, tabelas e campos no modo escuro adaptativo.
- Suavizado peso de fontes em menus, guias, botões e títulos.
- Ajustada topbar para recuperar harmonia visual:
  - busca menor;
  - empresa ativa proporcional e em linha;
  - serviços internos com labels completos em telas grandes e LEDs compactos em telas menores;
  - data/hora centralizadas por coluna;
  - regras responsivas por prioridade.
- Corrigido comportamento do menu sandwich em telas menores:
  - desktop preserva estado expandido/colapsado;
  - mobile usa drawer independente, sem contaminar o estado desktop.
- Produtos/Serviços, Fornecedores, Clientes, Centros de custo, Funções/Cargos e Departamentos passam a herdar o ícone da rota pelo BasePage.
- Splash ajustado para ficar visível em tempo real, sem pausa artificial no final.
- Scalar/API interna corrigidos para uso em rede:
  - iframe da documentação usa proxy same-origin quando a aplicação está no webport/browser;
  - Scalar servido via `/__internal_api/docs` passa a buscar `/__internal_api/openapi.json`.

## Validação executada

```bash
npm run build:web:tauri
```

Resultado: build web e cópia para `src-tauri/dist` concluídos com sucesso.

## Validação não executada

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Motivo: `cargo` não está instalado no ambiente atual.

## Preservado

- Regras de negócio.
- Algoritmos.
- Payloads.
- Endpoints.
- Stores sensíveis.
- Services sensíveis.
- Autenticação.
- Permissões.
- Sessão.
- Licenciamento.
- Persistência atual.
