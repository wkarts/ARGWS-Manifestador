# Implementação 1.1.12 - Web proxy alinhado ao School

Esta versão reimplementa o servidor Web/proxy do template usando o mesmo padrão operacional do School:

- servidor web dedicado em thread própria;
- inicialização idempotente por handle global;
- leitura de porta/host por env/default, sem depender de configuração persistida antiga;
- publicação da pasta `dist` gerada pelo Vite;
- fallback SPA para `index.html`;
- proxy `/__internal_api/*` para a API interna;
- tentativa de bind em `127.0.0.1` quando o bind LAN em `0.0.0.0` falha no Windows.

Portas padrão:

- API interna: 61001
- Web/proxy: 61002
- Webhook: 61003
- WebSocket: 61004
