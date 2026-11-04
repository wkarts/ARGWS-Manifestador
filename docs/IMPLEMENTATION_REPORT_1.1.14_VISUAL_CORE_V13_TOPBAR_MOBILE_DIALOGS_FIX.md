# Manifestador NF-e 1.1.14 — Visual Core v13

## Objetivo

Corrigir a harmonia final da topbar, melhorar responsividade da seleção de empresa/status dos serviços e substituir diálogos nativos do navegador/SO por diálogos visuais próprios da aplicação.

## Alterações aplicadas

- Ajustada a box de empresa ativa na topbar para uma linha única: `Empresa ativa: Empresa Demonstração`.
- Ampliada e proporcionada a largura da seleção de empresa em telas grandes.
- Em telas menores, seleção de empresa e status API/Webhook/WebSocket deixam a topbar e passam para o rodapé da sidebar/drawer, acima dos botões de Configurações e Sair.
- Mantido status dos serviços por LEDs, com hint individual.
- Criado serviço visual de diálogo em `src/services/dialog.ts`.
- Criado host global de diálogo em `src/components/base/AppDialogHost.vue`.
- Substituídos `confirm()` e `alert()` nativos por diálogos próprios do sistema.
- Atualizadas confirmações de exclusão em empresas, usuários, perfis, entidades, integrações, logs e temas próprios.
- Atualizados providers de notificação para usar diálogo próprio quando a notificação nativa não estiver disponível.

## Validação

Executado com sucesso:

```bash
npm run build:web:tauri
```

Não foi possível executar validação Rust neste ambiente porque `cargo` não está instalado.

## Preservado

- Regras de negócio.
- Algoritmos.
- Endpoints.
- Payloads.
- Autenticação.
- Permissões.
- Sessão.
- Licenciamento.
- API interna.
- Serviços Tauri/Rust.
- Persistência atual.
