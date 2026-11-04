# Manifestador NF-e Tauri Desktop Web 1.1.11 — School Proxy Real Fix

Esta versão corrige a publicação da mesma aplicação interna do Tauri no navegador seguindo o padrão real do School.

## Diagnóstico comparativo

No School, o servidor Web é iniciado pelo ciclo da API interna e usa host local por padrão, evitando disputa em `0.0.0.0` quando há algum serviço/processo escutando em outra interface da mesma porta. No Template, a porta Web estava tentando bind em `0.0.0.0:61002`, causando `os error 10048` mesmo quando `127.0.0.1:61002` não respondia no navegador.

## Correções

- Removida inicialização duplicada do Web/proxy no `app_bootstrap`.
- Removida inicialização separada do Web/proxy no `setup` do Tauri.
- O Web/proxy volta a ser iniciado pelo ciclo da API interna, como no School.
- Adicionado lock global de inicialização para impedir corrida de múltiplos starts simultâneos.
- Host padrão do Web/proxy alterado para `127.0.0.1`, igual ao padrão funcional do School.
- `MANIFESTADOR_NFE_WEB_BIND_LAN=false` por padrão.
- Para acesso em rede, configurar explicitamente `MANIFESTADOR_NFE_WEB_HOST=0.0.0.0` e `MANIFESTADOR_NFE_WEB_BIND_LAN=true`.
- Mantido proxy `/__internal_api/*` para a API interna.
- Mantido Scalar padrão, sem documentação customizada.

## URLs esperadas

Local:

```text
http://127.0.0.1:61002/
http://127.0.0.1:61002/__internal_api/health
http://127.0.0.1:61002/__internal_api/docs
```

Rede local, somente se configurado:

```env
MANIFESTADOR_NFE_WEB_HOST=0.0.0.0
MANIFESTADOR_NFE_WEB_BIND_LAN=true
```

## Observação

Se a porta `61002` estiver ocupada no próprio `127.0.0.1`, a aplicação informa erro. Nesse caso é necessário encerrar o processo que ocupa a porta ou alterar `MANIFESTADOR_NFE_WEB_PORT`.
