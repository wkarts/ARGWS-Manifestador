# Implementation Report — 1.1.14 Visual Core v13.5 WebPort Loopback/LAN Fix

## Objetivo

Corrigir o comportamento do WebPort/Proxy na porta `61002` quando o serviço fica acessível pelo IP da rede local, mas não responde por `localhost`/`127.0.0.1`.

## Correção aplicada

- Mantida a política de porta fixa `61002`.
- Mantido o comportamento de LAN/remoto quando configurado como `0.0.0.0`.
- Quando o bind principal em `0.0.0.0:61002` cai no fallback para o IP LAN específico, o servidor tenta iniciar também um listener auxiliar em `127.0.0.1:61002`.
- Se o loopback estiver ocupado por outro processo, a aplicação não quebra: registra aviso técnico no log e mantém o acesso pela LAN.
- O listener loopback auxiliar é encerrado junto com o runtime web principal.
- Evitado reinício desnecessário quando o runtime está ativo em IP LAN por fallback de configuração wildcard.

## Arquivo alterado

- `src-tauri/src/internal_api/web_server.rs`

## Impacto esperado

Quando possível, a porta `61002` passa a responder simultaneamente em:

- `http://127.0.0.1:61002`
- `http://localhost:61002`
- `http://<IP_DA_REDE_LOCAL>:61002`

## Preservado

- Sem alteração visual.
- Sem alteração de regra de negócio.
- Sem alteração de endpoints.
- Sem alteração de payloads.
- Sem alteração da API interna.
- Sem alteração na porta configurada.
- Sem fallback automático para outra porta.

## Validação

Não foi possível executar `cargo check`/`cargo clippy` neste ambiente porque a ferramenta `cargo` não está instalada.

O build web também não foi executado neste ambiente porque as dependências Node do projeto não estão instaladas (`vue-tsc` ausente). Como o patch altera apenas Rust, os diretórios `dist` e `src-tauri/dist` foram preservados.
