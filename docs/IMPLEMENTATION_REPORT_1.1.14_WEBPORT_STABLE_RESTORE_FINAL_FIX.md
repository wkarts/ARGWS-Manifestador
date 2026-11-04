# Manifestador NF-e Tauri 1.1.14 — Webport Stable Restore Final Fix

## Objetivo

Restaurar o comportamento funcional do Webport/Proxy sem depender de PowerShell, CMD ou scripts externos, mantendo a interface desktop carregando mesmo quando as portas locais estão ocupadas por múltiplas instâncias.

## Correções aplicadas

- O frontend voltou a solicitar a inicialização do `web_proxy_start` durante o startup, mas sem exibir erro fatal na interface.
- O Webport tenta primeiro publicar em `0.0.0.0:61002` quando `MANIFESTADOR_NFE_WEB_BIND_LAN=true`.
- Se o bind LAN falhar, o app tenta fallback local seguro em `127.0.0.1:61002` para não deixar o navegador local quebrado.
- Se a porta já estiver ocupada por outra instância válida do Manifestador NF-e, a nova instância reutiliza o Webport existente.
- Se a porta estiver ocupada por processo externo ou instância antiga não reconhecida, a interface desktop continua carregando normalmente e registra aviso nos logs, sem tela branca.
- O fallback de porta alternativa fica condicionado a `MANIFESTADOR_NFE_WEB_AUTO_PORT=true`.
- O router Vue permanece montado desde o primeiro frame e o splash não bloqueia a renderização da aplicação.

## Resultado esperado

- Primeira instância válida: tenta abrir `0.0.0.0:61002` para acesso local e LAN.
- Se LAN falhar mas localhost estiver livre: abre `127.0.0.1:61002` para manter o acesso local.
- Instâncias adicionais: reaproveitam a instância existente quando possível e não quebram a tela.
- O desktop não fica branco por falha de API/Webport.

## Observação

Se uma versão antiga estiver segurando `61002` apenas em `127.0.0.1`, nenhuma versão nova consegue transformar essa mesma porta em LAN enquanto o processo antigo estiver aberto. Nesse caso, feche as instâncias antigas pelo Gerenciador de Tarefas e abra esta versão corrigida.
