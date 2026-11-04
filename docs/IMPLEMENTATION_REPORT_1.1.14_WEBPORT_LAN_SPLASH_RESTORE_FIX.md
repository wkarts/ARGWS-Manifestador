# Patch 1.1.14 - Webport LAN e Splash Restaurado

## Objetivo

Corrigir a regressão onde o Webport voltava a funcionar somente em localhost e restaurar o splash screen de inicialização sem permitir tela branca.

## Correções

- O Webport prioriza sempre bind em `0.0.0.0` para acesso LAN/remoto.
- Removido fallback automático para `127.0.0.1` no mesmo porto, pois isso mascarava o problema e bloqueava LAN.
- `MANIFESTADOR_NFE_WEB_AUTO_PORT` passa a ser verdadeiro por padrão para evitar quebra quando `61002` estiver ocupado.
- Fallback localhost só acontece com `MANIFESTADOR_NFE_WEB_ALLOW_LOCALHOST_FALLBACK=true`.
- Splash screen visual foi restaurado como primeira tela.
- Timeout de segurança libera a interface caso algum serviço trave, evitando janela branca.

## Observação operacional

Se uma versão antiga estiver segurando `61002` em `127.0.0.1`, a nova versão não consegue transformar essa mesma porta em LAN. Nessa situação, a correção tenta abrir uma porta LAN alternativa (`61003`, `61004`, etc.) em vez de cair silenciosamente para localhost.
