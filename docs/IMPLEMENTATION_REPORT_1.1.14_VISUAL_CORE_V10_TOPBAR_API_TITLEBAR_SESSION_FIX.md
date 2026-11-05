# Implementação 1.1.14 — Visual Core v10

## Objetivo
Corrigir a harmonia da topbar, resolver URLs internas incorretas quando a aplicação é acessada via rede, padronizar a barra de título das páginas no mesmo modelo da Dashboard e reduzir a espera final do splash screen.

## Ajustes aplicados

- Topbar reorganizada em zonas fixas e responsivas.
- Campo de busca reduzido e com prioridade menor em telas menores.
- Empresa ativa mantida em linha única e proporcional.
- Indicadores API/Webhook/WebSocket mantidos compactos.
- Data/hora centralizadas internamente:
  - coluna 1: dia da semana e mês;
  - coluna 2: hora e data.
- Tooltip de data/hora corrigido para não repetir o dia da semana.
- URLs da API interna/Scalar normalizadas para o hostname do navegador quando acessado via rede.
- Documentação Scalar deixou de forçar caminho local incorreto quando em acesso LAN.
- Páginas de cadastro genérico, empresas, logs, Webhook e WebSocket foram migradas para `BasePage` com o mesmo padrão visual da Dashboard.
- CSS herdável reforçado para manter o mesmo TitleBar em páginas já existentes.
- Splash screen acelerado na etapa final, removendo sensação de travamento em 96%.
- Monitoramento de sessão protegido contra sobreposição de chamadas quando a rede estiver lenta.

## Segurança e sessão

O monitoramento de sessão continua sendo uma verificação leve no frontend, executada com `setInterval` quando há usuário autenticado. Não é cron do sistema operacional nem WebSocket. A chamada é ignorada na tela de login e protegida contra concorrência para evitar várias validações simultâneas.

## Validação

Executado com sucesso:

```bash
npm run build:web:tauri
```

Não executado:

```bash
cargo check
```

Motivo: o ambiente atual não possui `cargo` instalado.

## Preservado

- Regras de negócio.
- Algoritmos.
- Endpoints.
- Payloads.
- Services sensíveis.
- Stores sensíveis.
- Autenticação.
- Permissões.
- Sessão.
- Licenciamento.
- API interna.
- Tauri/Rust.
- Persistência atual.
