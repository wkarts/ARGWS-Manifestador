# Correção 1.1.4 — Proxy Web, Scalar interno e remoção de comandos externos em runtime

## Objetivo

Corrigir a execução da API interna e da documentação Scalar quando o backend é iniciado em `0.0.0.0`, permitir acesso local via navegador/proxy e remover a execução de comandos externos da aplicação em tempo de runtime.

## Correções aplicadas

- Normalização automática de URLs de cliente quando o bind da API interna usa `0.0.0.0`, expondo `127.0.0.1` para a UI local.
- Correção do `docs_url` e `base_url` retornados por `internal_api_status`.
- Correção do teste da API interna para nunca tentar acessar `http://0.0.0.0` como cliente.
- Correção da página Scalar para usar `/__internal_api/docs` quando estiver no navegador/Web via Vite proxy.
- Correção da página Scalar para usar `127.0.0.1` quando estiver no Tauri e a API estiver bindada em `0.0.0.0`.
- Inclusão de aliases diretos na API para `/__internal_api/health`, `/__internal_api/status`, `/__internal_api/openapi.json`, `/__internal_api/docs` e demais rotas principais.
- Inclusão de `/` na API interna redirecionando para `/docs` quando a documentação estiver habilitada.
- Remoção da execução de comandos externos dentro da aplicação em runtime para startup/serviços.

## Sobre serviços e inicialização com Windows

A aplicação não executa mais `cmd`, `PowerShell`, `reg.exe`, `sc.exe` ou `systemctl` em runtime para instalar/iniciar/parar serviços ou registrar startup. Esses pontos agora retornam resposta controlada informando que devem ser tratados pelo instalador, helper nativo dedicado ou pacote derivado.

## Validação realizada neste ambiente

```bash
npm run typecheck
npm run build:web
```

Ambos passaram com sucesso.

## Validação pendente no Windows do desenvolvedor

```bat
build-windows.bat
npm run tauri:build
```

A validação Rust/Tauri não foi executada neste ambiente porque `cargo` não está disponível.
