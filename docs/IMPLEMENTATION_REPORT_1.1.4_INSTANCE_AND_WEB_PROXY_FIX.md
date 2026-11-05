# Correção 1.1.4 — Instância única opcional e proxy Web LAN

## Objetivo

Esta correção estabiliza dois pontos críticos do template:

1. Evitar múltiplas instâncias da aplicação desktop causando conflito nas portas da API interna e do servidor Web local.
2. Fazer o servidor Web local ser iniciado também quando a API interna for iniciada manualmente ou via tray, seguindo o comportamento observado no projeto School.

## Instância única

Por padrão, o template agora bloqueia múltiplas instâncias do app Tauri. Se o usuário abrir o executável novamente, a janela existente é trazida para frente.

Configuração:

```env
MANIFESTADOR_NFE_ALLOW_MULTIPLE_INSTANCES=false
APP_ALLOW_MULTIPLE_INSTANCES=false
```

Para permitir múltiplas instâncias:

```env
MANIFESTADOR_NFE_ALLOW_MULTIPLE_INSTANCES=true
```

## Risco de múltiplas instâncias

Existe risco real de conflito quando a mesma aplicação sobe serviços locais, porque duas instâncias podem tentar ocupar as mesmas portas:

- API interna: 61001
- Web local/proxy: 61002
- serviços auxiliares: 61003
- bridge/core: 61004

Por isso, a configuração padrão segura é bloquear múltiplas instâncias.

## Proxy Web

O servidor Web local agora é iniciado também dentro do fluxo de `internal_api::server::start_background`, igual ao padrão funcional observado no School. Isso garante que iniciar a API interna também disponibilize a interface Web/proxy quando configurado.

Endereços esperados:

```text
http://127.0.0.1:61002/
http://IP_DA_MAQUINA:61002/
http://127.0.0.1:61002/__internal_api/health
http://IP_DA_MAQUINA:61002/__internal_api/docs
```

## Observação

O servidor Web continua sem depender de cmd, PowerShell, shell, reg.exe, sc.exe ou systemctl em runtime.
