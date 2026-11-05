# Implementação 1.1.14 — Visual Core v6: Dashboard, guias, menu do usuário e tema Laranja Escuro

## Tipo de entrega
Patch corretivo visual sobre a versão Visual Core v5.

## Correções aplicadas

- Adicionado novamente o tema **Laranja escuro**, preservando os temas atuais e os temas anteriores com sidebar/background coloridos.
- Corrigido o menu do usuário para ficar sempre sobre as guias/abas.
- Adicionado fechamento do menu do usuário ao clicar fora ou pressionar `Esc`.
- Adicionada área de personalização da dashboard, acessível pelo botão `Personalizar dashboard`.
- Ajustada a dashboard para manter os quatro painéis centrais na mesma linha quando houver espaço:
  - Saúde do sistema
  - Atalhos rápidos
  - Módulos do projeto
  - Últimos eventos
- Ajustado alinhamento das guias/abas e do botão `[+]`, mantendo mesma altura e espaçamento inferior uniforme.
- Mantida a sidebar recolhida como barra estreita com ícones no desktop.

## Validação

Executado com sucesso:

```bash
npm run build:web:tauri
```

## Áreas preservadas

- regras de negócio
- algoritmos
- services sensíveis
- stores sensíveis
- payloads
- endpoints
- autenticação
- permissões
- sessão
- licenciamento
- API interna
- Tauri/Rust
- persistência atual

## Branch sugerida

```txt
feature/visual-core-v6-dashboard-tabs-usermenu-theme-fix
```
