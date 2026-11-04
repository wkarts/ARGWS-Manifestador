# Correção 1.1.14 — Webport/proxy iniciado internamente pelo Tauri, sem cmd/PowerShell

## Objetivo

Remover a dependência de scripts `.bat`, PowerShell ou modo command line para iniciar o webport/proxy.

A porta web deve ser aberta pelo próprio processo Tauri Desktop durante a inicialização da aplicação.

## Correções

- O webport/proxy é iniciado no `setup()` do Tauri Desktop.
- O splash/startup também chama `web_proxy_start` como reforço interno via `invoke`, sem shell externo.
- Foram adicionados comandos Tauri nativos: `web_proxy_status`, `web_proxy_start`, `web_proxy_stop`, `web_proxy_restart`.
- O host padrão do webport foi consolidado para `127.0.0.1`, evitando falhas de bind LAN/0.0.0.0 no Windows.
- Se o host configurado for LAN e falhar, o runtime tenta `127.0.0.1` na mesma porta.
- O servidor web agora lê as configurações persistidas da aplicação em vez de depender apenas de `RuntimeSettings::default()`.
- Scripts auxiliares de command line para webport foram removidos do pacote.

## Resultado esperado

Após abrir o executável Tauri normalmente, sem scripts externos, a aplicação deve publicar:

- `http://127.0.0.1:61002/`
- `http://127.0.0.1:61002/__web_health`
- `http://127.0.0.1:61002/__internal_api/health`
- `http://127.0.0.1:61002/__internal_api/api/command`

## Observação

O build não abre porta. A porta só existe enquanto o processo Tauri estiver em execução.
