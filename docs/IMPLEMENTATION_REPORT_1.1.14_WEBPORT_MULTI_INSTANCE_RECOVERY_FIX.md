# Implementação 1.1.14 — Webport multi-instância/recovery fix

## Tipo
Patch corretivo sobre o Manifestador NF-e Tauri 1.1.14.

## Problema corrigido
O webport/proxy passou a usar bind estrito em `0.0.0.0:61002` para permitir acesso em rede local/remoto. Porém, quando já existia outra instância do Manifestador NF-e usando a porta `61002`, o Windows retornava `os error 10048` e a interface exibia erro no startup.

Isso gerava três efeitos ruins:

1. A aplicação exibia alerta vermelho ao iniciar.
2. O usuário precisava abrir várias instâncias até uma ficar utilizável.
3. O webport ficava instável quando uma instância anterior já estava hospedando a porta.

## Causa técnica
A versão anterior tratava `AddrInUse` como erro fatal do webport. Isso é correto quando a porta está ocupada por um processo externo desconhecido, mas é incorreto quando a porta já está ocupada por outra instância válida do próprio Manifestador NF-e.

Além disso, o frontend chamava `web_proxy_start` durante o splash screen, repetindo a tentativa de abrir a porta mesmo quando o setup nativo do Tauri já tinha tentado iniciar o serviço.

## Correções aplicadas

### 1. Inicialização idempotente do Webport
Arquivo:

```text
src-tauri/src/internal_api/web_server.rs
```

Quando o bind em `0.0.0.0:61002` retorna `AddrInUse`, a aplicação agora consulta:

```text
http://127.0.0.1:61002/__web_health
```

Se o endpoint responder como `manifestador-nfe-web-proxy`, a nova instância reutiliza o webport existente e retorna status de sucesso com:

```json
{
  "running": true,
  "external": true
}
```

Assim, o app não quebra por tentar abrir uma segunda porta.

### 2. Inicialização idempotente da API interna
Arquivo:

```text
src-tauri/src/internal_api/server.rs
```

O mesmo conceito foi aplicado à API interna. Se `61001` já estiver ocupada por uma API do próprio Manifestador NF-e, a nova instância reutiliza a API existente em vez de falhar.

### 3. Remoção da chamada redundante de start no splash
Arquivo:

```text
src/services/startup.ts
```

O splash não chama mais:

```text
web_proxy_start
```

Agora chama apenas:

```text
web_proxy_status
```

O servidor web/proxy deve ser iniciado pelo setup nativo do Tauri, não pelo frontend durante o carregamento visual.

### 4. Dist pré-compilado ajustado
Arquivos:

```text
dist/assets/index-*.js
src-tauri/dist/assets/index-*.js
```

O pacote segue com o comportamento do frontend ajustado também no bundle gerado.

## Comportamento esperado

### Primeira instância
A primeira instância válida deve abrir:

```text
API interna: 0.0.0.0:61001
Webport/proxy: 0.0.0.0:61002
```

### Segunda instância
A segunda instância não deve quebrar a interface. Ela detecta os serviços já ativos e segue carregando a tela normalmente.

### Acesso local
```text
http://127.0.0.1:61002
```

### Acesso em rede
```text
http://IP_DA_MAQUINA:61002
```

## Observação importante
Se uma versão antiga do Manifestador NF-e já estiver aberta usando `61002` apenas em `127.0.0.1`, nenhuma nova versão consegue transformar essa porta em LAN enquanto o processo antigo estiver vivo. Nesse caso, feche todas as instâncias antigas pelo Gerenciador de Tarefas e abra apenas a versão corrigida.

Depois disso, a primeira instância corrigida deverá abrir o bind em `0.0.0.0:61002`.
