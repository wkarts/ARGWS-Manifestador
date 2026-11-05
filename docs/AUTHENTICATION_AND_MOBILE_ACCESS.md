# Autenticação, senha e acesso mobile híbrido

## Objetivo

A template separa o contrato do desktop do contrato mobile sem duplicar regras de segurança. O backend Rust é a fonte de verdade para senha, sessão, perfil, permissões e empresas autorizadas.

## Primeiro acesso do usuário master

Na primeira inicialização o backend cria o usuário master com uma credencial aleatória mantida somente em memória durante o seed. Nenhuma senha temporária é gravada no frontend ou em arquivo.

No desktop e no modo mobile local com autenticação obrigatória, a rota `/primeiro-acesso` permite definir a primeira senha sem exigir conhecimento da credencial interna.

Em instalações headless, defina antes do primeiro startup:

```env
MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_USERNAME=admin
MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD=SenhaInicialControlada123
```

Quando a senha é fornecida por ambiente, ela é tratada como definitiva. Os arquivos `.bootstrap-admin.local` eventualmente deixados por versões antigas são apenas detectados para limpeza e nunca são recriados.

Em upgrades, um administrador legado já existente é promovido a master sem rotação automática de sua senha. A variável de ambiente substitui a credencial somente quando esse master ainda está marcado com senha provisória. Isso preserva instalações em uso e torna a implantação headless determinística.

## Política de senha

A política é persistida em `app_settings`, validada pelo Rust e compartilhada por primeiro acesso, troca de senha e manutenção de usuários.

Padrão:

```text
mínimo: 8
máximo: 128
letra: obrigatória
número: obrigatório
maiúscula: opcional
minúscula: opcional
símbolo: opcional
```

O master pode reduzir o mínimo até 6 caracteres e ativar ou desativar os requisitos individualmente.

## Modos do aplicativo mobile

### Local independente

- usa SQLite privado do dispositivo;
- pode entrar diretamente com usuário local técnico;
- pode exigir login local por opção do produto;
- não inicia tray, Scalar, API HTTP, Webhook Service, WebSocket Service ou serviço do sistema;
- mantém dashboards, formulários e gráficos no próprio aplicativo.

### Cliente remoto

- configura a URL do desktop, headless ou CloudPanel;
- autentica em `POST /api/auth/login` com canal `mobile`;
- executa comandos em `POST /api/command`;
- usa sessão individual do usuário;
- aplica `api_access_enabled`, `mobile_access_enabled`, permissões e empresas vinculadas;
- permite token global do gateway como camada adicional; esse token fica apenas em `sessionStorage`.

Para acesso a partir do smartphone, configure explicitamente bind de rede, CORS, TLS e firewall no servidor. Não exponha a API sem autenticação e sem HTTPS em redes não confiáveis.

## Campos de acesso do usuário

```text
api_access_enabled
mobile_access_enabled
api_session_timeout_minutes
profile_ids
empresa_ids
```

Permissões específicas:

```text
api:access
mobile:access
```

O canal da sessão é armazenado como `desktop`, `api`, `mobile` ou `mobile-local`.

## Escopo por empresa

A API filtra empresas e o combo de empresas pelo vínculo em `usuarios_empresas`. Leitura, alteração e exclusão fora do escopo são rejeitadas. O master mantém acesso geral.

As entidades genéricas da template não possuem obrigatoriamente `empresa_id`. Cada aplicação derivada deve declarar quais tabelas são multiempresa e aplicar o mesmo escopo em seus repositórios. A template não injeta esse campo automaticamente para evitar alteração indevida de modelos existentes.

## Endpoints de autenticação

```text
POST /api/auth/login
POST /api/auth/restore
POST /api/auth/logout
POST /api/auth/change-password
POST /api/command
```

O token global configurado em `MANIFESTADOR_NFE_API_TOKEN` protege o gateway. O `session_token` identifica o usuário e nunca é substituído pelo token global.
