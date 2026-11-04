# Manifestador NF-e Tauri Desktop Web 1.0.2 — Correções concluídas

## Status

Versão corrigida para uso como template único Web/PWA/Tauri, mantendo a base existente e aplicando correções de estabilidade, branding, menu, tray, API interna e cadastro de usuários.

## Correções aplicadas

- Corrigida a separação de portas:
  - Web navegador: `npm run dev` em `http://localhost:61002`.
  - Tauri dev: `npm run tauri:dev`, com Vite interno em `http://localhost:1420`.
- Corrigida inicialização para evitar tela branca permanente quando bootstrap/sessão falharem ou demorarem.
- Criada camada central `src/config/appBranding.ts` para reduzir resíduos do template em aplicações herdadas.
- Reorganizado o menu:
  - Dashboard;
  - Cadastro;
  - Sistema;
  - Ferramentas;
  - Documentação.
- Todos os módulos opcionais do template foram pré-ativados em `projectConfig.ts`.
- Corrigido `combo_list("empresas")` no WebProvider e no backend Rust/Tauri.
- Adicionada foto no cadastro de usuários.
- Exibição da foto/nome do usuário na sidebar.
- Removida exibição do login na sidebar.
- Corrigidas as faixas verdes indevidas na dashboard em Banco de dados e Licenciamento.
- Adicionada tela operacional da API interna com:
  - host;
  - porta;
  - status;
  - iniciar;
  - parar;
  - reiniciar;
  - abrir Scalar fora;
  - visualizar Scalar dentro da aplicação.
- Scalar mantido em tema claro via HTML/CSS da documentação interna.
- Tray icon melhorado:
  - usa ícone padrão da aplicação;
  - menu de restaurar;
  - iniciar API interna;
  - reiniciar API interna;
  - parar API interna;
  - sair definitivamente.
- Diretório de dados agora pode ser herdado em apps derivados via variáveis:
  - `MANIFESTADOR_NFE_LOCAL_DATA_DIR`;
  - `APP_LOCAL_DATA_DIR`.
- Nome/identificador runtime podem ser herdados via:
  - `MANIFESTADOR_NFE_NAME` / `APP_NAME`;
  - `MANIFESTADOR_NFE_IDENTIFIER` / `APP_IDENTIFIER`.

## Validação executada neste ambiente

```bash
npm ci --ignore-scripts
npm run typecheck
npm run build:web
```

A validação Rust/Tauri deve ser executada em ambiente com Rust/Cargo instalado:

```bash
npm run tauri:dev
npm run tauri:build
npm run fmt:rust:check
npm run lint:rust
```

## Comandos de uso

### Navegador

```bash
npm ci
npm run dev
```

Acesse:

```text
http://localhost:61002
```

### Navegador em outra porta

```bash
VITE_DEV_PORT=5180 npm run dev
```

No Windows PowerShell:

```powershell
$env:VITE_DEV_PORT="5180"; npm run dev
```

### Tauri

```bash
npm run tauri:dev
```

### Build Web

```bash
npm run build:web
```

## PR sugerido

Branch:

```text
fix/template-desktop-web-runtime-ux-stability
```

Título:

```text
fix(template): stabilize desktop-web runtime, branding, tray and user page
```

Descrição:

```text
Este PR estabiliza o manifestador-nfe como base única para Web/PWA/Tauri, corrigindo inicialização, portas de desenvolvimento, branding herdável, menu, tray icon, API interna, Scalar, cadastro de usuários e compatibilidade de diretórios em aplicações derivadas.
```
