# Implementação 1.1.14 — LAN bind dos serviços e splash realtime

## Objetivo

Corrigir os cenários em que apenas a API interna funcionava em rede, enquanto Webport/Proxy, Webhook Service e WebSocket Service ficavam acessíveis apenas via localhost ou falhavam ao trocar o nível de acesso para rede local/remoto.

Também restaura o splash screen como etapa perceptível/realtime, com progresso visual mínimo antes da liberação da interface.

## Causa técnica

No Windows, quando uma porta já está presa em `127.0.0.1:PORTA`, uma tentativa posterior de bind em `0.0.0.0:PORTA` pode retornar `os error 10048`, mesmo que a intenção seja publicar o serviço em rede.

A API interna funcionava porque subia primeiro e conseguia prender o socket. Os serviços auxiliares podiam falhar porque:

- tentavam trocar de `127.0.0.1` para `0.0.0.0` com a mesma porta já presa;
- adotavam uma instância localhost antes de tentar publicar em LAN;
- não tinham alternativa de bind por interface de rede mantendo a mesma porta;
- o splash era desmontado assim que a inicialização terminava, ficando imperceptível.

## Correções aplicadas

### 1. Helper central de bind

Criado:

```text
src-tauri/src/net_bind.rs
```

Esse helper respeita portas fixas e não troca `61002` para `61003/61004`.

Quando o operador seleciona rede local/remoto e o bind em `0.0.0.0:PORTA` falha por conflito com loopback, o helper tenta a mesma porta no IPv4 principal da interface de rede, por exemplo:

```text
192.168.0.25:61002
```

Isso evita o falso bloqueio causado por listener anterior em `127.0.0.1:61002`, sem alterar a porta configurada.

### 2. Webport/Proxy

Alterado:

```text
src-tauri/src/internal_api/web_server.rs
```

Agora o Webport:

- tenta `0.0.0.0:61002` quando configurado como rede;
- se houver conflito com loopback, tenta a mesma porta na interface LAN real;
- reaproveita instância existente quando ela realmente responde como Manifestador NF-e;
- não bloqueia a janela desktop em caso de falha.

### 3. Webhook Service

Alterado:

```text
src-tauri/src/native_webhook/server.rs
```

Agora o Webhook:

- não adota antecipadamente uma instância localhost quando a configuração pede rede;
- tenta publicar na interface correta;
- mantém a porta fixa configurada;
- reaproveita instância existente apenas depois de tentar bind correto.

### 4. WebSocket Service

Alterado:

```text
src-tauri/src/native_websocket/server.rs
```

Mesmo comportamento do Webhook, respeitando host/porta/path configurados.

### 5. Splash realtime

Alterados:

```text
index.html
src/App.vue
src/services/startup.ts
```

O splash agora:

- nasce já no HTML inicial com barra de progresso;
- permanece visível por no mínimo 6,5 segundos;
- continua atualizando mensagens e progresso;
- não bloqueia a interface indefinidamente se algum serviço demorar.

## Observação operacional

`0.0.0.0` é endereço de escuta, não URL de navegação.

Localmente use:

```text
http://127.0.0.1:61002
```

Em rede use:

```text
http://IP_DA_MAQUINA:61002
```

Se a porta já estiver ocupada por processo externo, a aplicação não derruba esse processo e não troca a porta automaticamente.
