# IMPLEMENTATION REPORT 1.1.14 - Splash Realtime + LAN Bind Stability

Este patch corrige regressões pontuais sem refatoração abrupta.

## Correções aplicadas

1. Splash screen realtime preservado:
   - Corrigido conflito de CSS que sobrescrevia `.startup-splash` de `fixed` para `relative`.
   - A camada de splash volta a ficar acima da aplicação durante a inicialização.
   - Tempo mínimo visível ajustado para 4,5 segundos.
   - Fallback HTML inicial agora mostra barra de progresso antes do Vue montar.

2. Bind Local/Rede estabilizado:
   - O Webport não reaproveita listener em memória se host/porta configurados mudaram.
   - Troca de `127.0.0.1` para `0.0.0.0` agora para o listener antigo e tenta subir novamente.
   - Adicionado retry curto antes de declarar porta ocupada, evitando falso conflito durante restart.
   - `is_port_available` passa a testar o host real solicitado, inclusive `0.0.0.0`.

3. Webhook/WebSocket:
   - Não reaproveitam handle antigo quando host/porta/path mudam.
   - Aplicado retry curto para bind fixo.

4. Página de parâmetros:
   - Ajuste de rolagem para impedir corte da parte inferior da tela.

## Observação técnica

`0.0.0.0` é apenas endereço de bind/escuta. A URL local continua sendo `127.0.0.1`, e a URL em rede deve usar o IP da máquina.

Se `0.0.0.0:61002` continuar com `os error 10048`, existe outro processo/instância segurando a mesma porta. Este patch evita falso conflito dentro do mesmo processo, mas não pode derrubar outro processo externo sem permissão do sistema.
