# Implementação 1.1.4 — Manifestador NF-e Tauri Desktop Web

## Objetivo

Evoluir o `manifestador-nfe` para uma base única Web/PWA/Tauri com foco em portabilidade real de aplicações derivadas, estabilidade de inicialização, abertura de páginas em abas/guias, serviços configuráveis por interface e `.env`, API interna acessível por proxy e documentação Scalar embutida com suporte a token.

## Pacotes analisados

- `manifestador-nfe-1.1.3_atual.zip`: base principal utilizada.
- `school-desktop-suite-template-web-1.1.3-etapa46.zip`: referência para workspace com abas/guias persistentes.
- `ALL-DRIVER-PLATFORM-1.8.0-native-camera-rtsp-server-v109.zip`: referência arquitetural para serviços acessíveis por localhost, rede local e cenário externo por host/porta configurável.

## Principais recursos portados/adicionados

### 1. Abas/guias de páginas

Foi portada a abordagem operacional do pacote `school` para o template:

- abertura de páginas em guias;
- persistência das guias no storage local;
- guia ativa restaurável;
- fechamento individual;
- fechamento das demais guias com duplo clique;
- ocultação automática em ambiente mobile quando configurado;
- configuração centralizada em `projectConfig.workspace`.

### 2. Execução Web/PWA com proxy para API interna

O `vite.config.ts` agora suporta:

- porta Web configurável;
- host Web configurável;
- proxy `/__internal_api` para encaminhar chamadas à API interna;
- configuração por `.env`/variáveis de ambiente.

Portas sugeridas:

- `61001`: API interna;
- `61002`: Web/Vite;
- `61003`: serviços auxiliares;
- `61004`: bridge/core.

A porta padrão do Tauri devUrl foi preservada em `1420`.

### 3. Scalar interno com token

A documentação Scalar foi ajustada para carregar dentro da aplicação em tema claro, inclusive quando a API exige token.

Estratégia aplicada:

- Scalar recebe URL de OpenAPI com `?token=...`;
- `token_guard` da API interna aceita token por header ou querystring;
- o iframe interno consegue carregar `/docs` e `/openapi.json` sem depender de header customizado, que navegador não injeta facilmente em iframe.

### 4. Serviços configuráveis

A tela de sistema/API interna agora centraliza:

- host da API;
- porta da API;
- base URL;
- path da documentação;
- token;
- header do token;
- CORS;
- acesso por rede pública/local;
- iniciar/parar/reiniciar API;
- configuração de tray;
- iniciar com Windows;
- nome do serviço;
- auto start de serviços.

As mesmas opções também possuem variáveis no `.env.model`.

### 5. Correção de inicialização com tela branca

O bootstrap do frontend foi reorganizado para evitar bloqueios no primeiro carregamento:

- o Vue é montado imediatamente;
- o `router.isReady()` não bloqueia a renderização inicial;
- o splash possui timeout de recuperação;
- falhas de startup viram banner não bloqueante em vez de tela branca permanente.

### 6. Criação de aplicações derivadas

Foi criado o fluxo de manifesto para novos apps derivados:

- `app.manifest.example.json` como modelo;
- `scripts/create-derived-app.mjs` para aplicar identidade;
- troca de nome do app;
- troca de identificador;
- troca de diretório local;
- troca de package name;
- troca de versão;
- troca de publisher/desenvolvedor;
- cópia de ícones/logos quando informados;
- geração de `.env` base.

O objetivo é evitar resquícios como `manifestador_nfe` em apps derivados.

## Arquivos principais alterados/adicionados

- `src/main.ts`
- `src/App.vue`
- `src/layouts/AppLayout.vue`
- `src/config/appBranding.ts`
- `src/config/projectConfig.ts`
- `src/config/navigation.ts`
- `src/pages/SystemPage.vue`
- `src/pages/optional/InternalApiPage.vue`
- `src/pages/optional/ScalarDocsPage.vue`
- `src/core/invoker/WebCommandProvider.ts`
- `src/services/crud.ts`
- `vite.config.ts`
- `package.json`
- `.env.model`
- `app.manifest.example.json`
- `scripts/dev-web.mjs`
- `scripts/print-env-model.mjs`
- `scripts/create-derived-app.mjs`
- `src-tauri/src/runtime_config.rs`
- `src-tauri/src/internal_api/config.rs`
- `src-tauri/src/internal_api/routes.rs`
- `src-tauri/src/internal_api/docs.rs`
- `src-tauri/src/commands/runtime.rs`
- `src-tauri/src/commands/app.rs`
- `src-tauri/src/lib.rs`

## Validações executadas

Executado com sucesso:

```bash
npm ci --ignore-scripts
npm run typecheck
npm run build:web
node scripts/print-env-model.mjs
```

## Validação pendente no ambiente do desenvolvedor

Este ambiente não possui `cargo`, portanto os comandos Rust/Tauri precisam ser executados localmente:

```bash
npm run tauri:dev
npm run tauri:build
npm run fmt:rust:check
npm run lint:rust
```

## Observação técnica

A arquitetura foi preparada para que todo serviço que venha a ser disponibilizado pela aplicação tenha configuração por interface e por `.env`, preservando a porta padrão do Tauri devUrl separada do restante dos serviços.
