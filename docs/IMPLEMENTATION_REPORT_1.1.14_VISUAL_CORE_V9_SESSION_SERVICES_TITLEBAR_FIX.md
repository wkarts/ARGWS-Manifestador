# Manifestador NF-e 1.1.14 — Visual Core v9

## Objetivo

Correção incremental da remodelagem visual com foco em topbar, titlebar herdável, responsividade, status real de serviços auxiliares via rede e segurança de sessão ativa.

## Correções aplicadas

- Data/hora da topbar reorganizada com alinhamento central por coluna:
  - `Quarta-feira` sobre `Junho`;
  - `02:12:28` sobre `03/06/2026`.
- Tooltip de data/hora corrigido para não repetir o nome do dia da semana.
- Empresa ativa ajustada para rótulo e nome da empresa na mesma linha.
- Campo de busca global e elementos da topbar compactados para reduzir quebra em telas menores.
- Comportamento da sidebar separado por viewport:
  - desktop mantém comportamento expandido/colapsado;
  - telas menores usam drawer aberto pelo botão sandwich;
  - o comportamento mobile não contamina a preferência visual do desktop.
- Padrão de barra de título visual da Dashboard reaplicado por CSS herdável nas páginas legadas que usam `.toolbar` ou `.page-heading`.
- Splash realtime acelerado para não ficar parado em 96% antes de exibir login/interface.
- Status de Webhook e WebSocket via rede corrigido:
  - API interna web agora retorna status real dos serviços nativos quando disponível no mesmo runtime;
  - frontend faz probe de `/health` via host atual do navegador quando o serviço está acessível pela rede;
  - páginas administrativas de Webhook/WebSocket montam URLs com o hostname do navegador, evitando `127.0.0.1` incorreto em acesso remoto.
- Segurança de sessão reforçada:
  - monitoramento ativo da sessão a cada 5 segundos;
  - quando o login em outro dispositivo derruba a sessão atual, o frontend limpa a sessão e redireciona para `/login`;
  - falhas por sessão inválida/expirada em comandos também forçam retorno ao login.

## Pontos preservados

- Regras de negócio.
- Algoritmos existentes.
- Payloads e endpoints existentes.
- Autenticação e permissões.
- Licenciamento.
- Persistência atual.
- Serviços principais e fluxo atual de runtime.

## Validação web

Executado com sucesso:

```bash
npm run build:web:tauri
```

Observação: `cargo` não está disponível neste ambiente, então a validação Rust local não pôde ser executada aqui.
