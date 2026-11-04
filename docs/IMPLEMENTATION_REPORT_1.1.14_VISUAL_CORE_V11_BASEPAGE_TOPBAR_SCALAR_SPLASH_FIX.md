# Manifestador NF-e 1.1.14 — Visual Core v11

## Objetivo

Aplicar correções finais de organização visual, padronização real das páginas em BasePage, harmonia da topbar, correção de URLs da API interna/Scalar em acesso via rede e ajuste do comportamento de splash/session monitor.

## Alterações aplicadas

- Perfis de acesso convertido para BasePage real.
- Cadastro de usuários convertido para BasePage real.
- Licenciamento convertido para BasePage real.
- BasePage passa a resolver o ícone pelo contexto da rota quando o ícone não for informado.
- TitleBar de BasePage usa o mesmo padrão visual da Dashboard, sem depender de adaptações de toolbar antiga.
- Topbar reorganizada para evitar quebra e sobreposição visual.
- Empresa ativa em linha única com melhor largura em desktop.
- Status API/Webhook/WebSocket com fonte menos pesada e ajuste responsivo.
- Data/hora alinhadas internamente por coluna e tooltip sem repetição do dia da semana.
- Scalar/Documentação da API interna normaliza host/porta para acesso via rede, evitando apontar para proxy/local incorreto quando a aplicação é aberta por IP LAN.
- Monitoramento de sessão ajustado para 10 segundos e protegido contra chamadas concorrentes.
- Splash mantido realtime: sem delay artificial e sem freio final.
- Modo escuro adaptativo ao tema, usando superfície ligeiramente mais clara que a sidebar para manter harmonia visual.
- Peso de fontes suavizado em guias, botões e menus para manter visual clean e moderno.

## Segurança preservada

- Regras de negócio preservadas.
- Payloads preservados.
- Endpoints preservados.
- Stores sensíveis preservadas.
- Services sensíveis preservados.
- Autenticação, sessão, permissões e licenciamento preservados.
- Tauri/Rust não alterados.

## Validação

Executado com sucesso:

```bash
npm run build:web:tauri
```

O ambiente não possui cargo instalado, portanto a validação Rust não foi executada neste pacote.
