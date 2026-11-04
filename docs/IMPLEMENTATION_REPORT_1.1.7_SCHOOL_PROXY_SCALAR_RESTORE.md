# Manifestador NF-e Tauri Desktop Web 1.1.7

## Objetivo

Corrigir a tentativa anterior de implementação do proxy Web, alinhando o comportamento ao padrão operacional do `school-desktop-suite-template-web`, sem transformar a documentação da API em página customizada.

## Alterações

- Restauração da documentação Scalar padrão baseada em `@scalar/api-reference`.
- Remoção da documentação local customizada que substituía o padrão Scalar.
- Servidor Web/proxy em Rust preservado na porta configurada `61002`.
- Publicação da mesma aplicação interna do Tauri no navegador.
- Proxy `/__internal_api/*` para a API interna `61001`.
- Build Web sem minificação, com `base: "./"`, `sourcemap: true` e assets locais da aplicação.
- Empacotamento limpo usando `src-tauri/dist` como resource, evitando `_up_/dist`.
- Ajuste do WebSocket Service para reaproveitar serviço existente saudável na mesma porta quando aplicável.
- Adição de cliente de teste WebSocket na tela administrativa.

## Portas

- `61001`: API interna REST/OpenAPI/Scalar.
- `61002`: Web App + proxy `/__internal_api/*`.
- `61003`: Webhook Service.
- `61004`: WebSocket Service.

## Observação

A aplicação não executa comandos externos de runtime para matar processos ou manipular serviços do Windows. Se uma porta estiver ocupada por processo externo inválido, a correção segura é encerrar o processo ou alterar a porta por `.env`/parâmetros.
