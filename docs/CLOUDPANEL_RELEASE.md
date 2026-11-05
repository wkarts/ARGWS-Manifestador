# Release CloudPanel/Linux sem instalador

Este release mantém a aplicação como está: frontend Vue/Vite + binário Rust/Tauri executando em modo headless/CLI/worker. No CloudPanel, o Node.js é usado apenas como launcher (`npm start`) para iniciar o binário Linux.

## Alvos oficiais Debian/Ubuntu

O build CloudPanel gera dois pacotes obrigatórios:

- `linux-x64`: `x86_64-unknown-linux-gnu`
- `linux-x86`: `i686-unknown-linux-gnu`

O pacote x64 é o alvo principal para servidores atuais. O pacote x86 é mantido para servidores 32 bits/legados e exige bibliotecas multiarch i386 no ambiente de build.

## Release pelo GitHub Actions

Workflow:

```txt
.github/workflows/cloudpanel-linux-release.yml
```

Ele foi preparado para:

1. compilar o frontend;
2. compilar o binário Linux x64;
3. compilar o binário Linux x86;
4. montar os pacotes `.tar.gz` sem instalador;
5. gerar `.sha256`;
6. subir tudo como artefato da execução.

O release oficial é coordenado por:

```text
.github/workflows/release.yml
```

Esse fluxo executa somente após o CI aprovado em `main`, incorpora os pacotes
CloudPanel ao conjunto multiplataforma, gera manifesto e checksums globais e
mantém tudo em um único draft até a matriz ficar completa ou uma publicação
parcial ser autorizada explicitamente.

Artefatos esperados:

```txt
manifestador-nfe-cloudpanel-v1.2.1-linux-x64.tar.gz
manifestador-nfe-cloudpanel-v1.2.1-linux-x64.tar.gz.sha256
manifestador-nfe-cloudpanel-v1.2.1-linux-x86.tar.gz
manifestador-nfe-cloudpanel-v1.2.1-linux-x86.tar.gz.sha256
```

### Build manual

Em **Actions > CloudPanel Linux Release > Run workflow**:

- `target`: `all`, `x64` ou `x86`.

O workflow manual não grava no GitHub Release. Isso evita concorrência com o
publicador coordenado.

## Preparar ambiente Debian/Ubuntu local

Em Debian 12, Ubuntu 22.04 ou Ubuntu 24.04:

```bash
bash scripts/linux/install-cloudpanel-build-deps.sh --all
```

Depois instale Node.js 22.14+ ou 24.10+ e Rust, caso ainda não existam no
ambiente.

## Gerar ambos os pacotes localmente

```bash
npm run build:linux:cloudpanel
```

Saída:

```txt
release/cloudpanel/manifestador-nfe-cloudpanel-v1.2.1-linux-x64.tar.gz
release/cloudpanel/manifestador-nfe-cloudpanel-v1.2.1-linux-x64.tar.gz.sha256
release/cloudpanel/manifestador-nfe-cloudpanel-v1.1.14-linux-x86.tar.gz
release/cloudpanel/manifestador-nfe-cloudpanel-v1.1.14-linux-x86.tar.gz.sha256
```

## Gerar individualmente

```bash
npm run build:linux:cloudpanel:x64
npm run build:linux:cloudpanel:x86
```

## Gerar via Docker

```bash
npm run build:linux:cloudpanel:docker
```

O Docker usa Debian Bookworm como base de build, instala as dependências x64 e i386, compila o frontend, compila os binários e monta os `.tar.gz` finais.

## Deploy no CloudPanel

No servidor:

```bash
tar -xzf manifestador-nfe-cloudpanel-v1.1.14-linux-x64.tar.gz
cd manifestador-nfe-cloudpanel-v1.1.14-linux-x64
cp .env.example .env
chmod +x bin/manifestador_nfe *.sh
npm start
```

No template Node.js Application do CloudPanel, use:

```bash
npm start
```

O `server.mjs` lê `.env`, usa `PORT` do CloudPanel como WebPort quando disponível, inicia o binário em modo headless e mantém API/WebPort/serviços acessíveis conforme configuração.

## Execução direta por terminal

```bash
./start.sh
./status.sh
./logs.sh
./stop.sh
./restart.sh
./ports.sh
```

CLI:

```bash
./cli.sh
```

Worker:

```bash
./worker.sh
```

Cron opcional:

```cron
* * * * * cd /caminho/do/app && ./check.sh >/dev/null 2>&1
```

## Portas configuráveis

| Serviço | Host | Porta |
|---|---:|---:|
| API Headless | `MANIFESTADOR_NFE_API_HOST` | `MANIFESTADOR_NFE_API_PORT` |
| Web/browser/WebPort | `MANIFESTADOR_NFE_WEB_HOST` | `MANIFESTADOR_NFE_WEB_PORT` |
| Webhook | `MANIFESTADOR_NFE_WEBHOOK_HOST` | `MANIFESTADOR_NFE_WEBHOOK_PORT` |
| WebSocket | `MANIFESTADOR_NFE_WEBSOCKET_HOST` | `MANIFESTADOR_NFE_WEBSOCKET_PORT` |

Padrão seguro:

```env
MANIFESTADOR_NFE_API_HOST=127.0.0.1
MANIFESTADOR_NFE_API_PORT=61001
MANIFESTADOR_NFE_WEB_HOST=127.0.0.1
MANIFESTADOR_NFE_WEB_PORT=61002
MANIFESTADOR_NFE_WEBHOOK_HOST=127.0.0.1
MANIFESTADOR_NFE_WEBHOOK_PORT=61003
MANIFESTADOR_NFE_WEBSOCKET_HOST=127.0.0.1
MANIFESTADOR_NFE_WEBSOCKET_PORT=61004
```

Para trocar portas:

```env
MANIFESTADOR_NFE_API_PORT=62001
MANIFESTADOR_NFE_WEB_PORT=62002
MANIFESTADOR_NFE_WEBHOOK_PORT=62003
MANIFESTADOR_NFE_WEBSOCKET_PORT=62004
```

## Publicar todas as portas

Use o modelo público:

```bash
cp .env.public.example .env
```

Ou ajuste manualmente:

```env
MANIFESTADOR_NFE_PUBLISH_ALL_PORTS=true
MANIFESTADOR_NFE_START_ALL_PORTS=true
MANIFESTADOR_NFE_RESPECT_CLOUDPANEL_PORT=false
MANIFESTADOR_NFE_PUBLIC_BIND_HOST=0.0.0.0

MANIFESTADOR_NFE_API_HOST=0.0.0.0
MANIFESTADOR_NFE_API_PORT=61001

MANIFESTADOR_NFE_WEB_HOST=0.0.0.0
MANIFESTADOR_NFE_WEB_PORT=61002

MANIFESTADOR_NFE_WEBHOOK_ENABLED=true
MANIFESTADOR_NFE_WEBHOOK_AUTO_START=true
MANIFESTADOR_NFE_WEBHOOK_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBHOOK_PORT=61003

MANIFESTADOR_NFE_WEBSOCKET_ENABLED=true
MANIFESTADOR_NFE_WEBSOCKET_AUTO_START=true
MANIFESTADOR_NFE_WEBSOCKET_HOST=0.0.0.0
MANIFESTADOR_NFE_WEBSOCKET_PORT=61004
```

Atenção: ao publicar portas diretamente, configure firewall, tokens e proxy HTTPS antes de expor em produção.

## Observação sobre x86

O build x86 usa `i686-unknown-linux-gnu`. Em host x64, ele precisa de multiarch i386, incluindo `libwebkit2gtk-4.1-dev:i386`. Se a distribuição não fornecer essas bibliotecas, compile o x86 em um ambiente i386 dedicado ou use o pacote x64.
