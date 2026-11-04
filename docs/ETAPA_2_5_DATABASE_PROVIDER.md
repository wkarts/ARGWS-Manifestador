# Etapa 2.5 — DatabaseProvider CRUD multi-banco real

Esta etapa inicia o CRUD genérico provider-based para operar sobre SQLite, MySQL/MariaDB e PostgreSQL sem duplicar regra de negócio na camada de comandos Tauri.

## Escopo aplicado

- SQLite permanece como banco padrão e funcional do desktop.
- `provider_entity_list`, `provider_entity_get`, `provider_entity_create`, `provider_entity_update` e `provider_entity_delete` passam a usar `DatabaseProvider`.
- O CRUD antigo baseado em `rusqlite` foi preservado para compatibilidade.
- MySQL/MariaDB usa a feature Rust `mysql-db`.
- PostgreSQL usa a feature Rust `postgres-db`.
- Firebird permanece fora do escopo funcional por compatibilidade.

## Seleção do driver

O provider usa as variáveis:

```bash
MANIFESTADOR_NFE_DATABASE_DRIVER=sqlite
# ou
MANIFESTADOR_NFE_DATABASE_DRIVER=mysql
# ou
MANIFESTADOR_NFE_DATABASE_DRIVER=postgres
```

Também é aceito `MANIFESTADOR_NFE_DB_DRIVER` como fallback.

## MySQL/MariaDB

Compile com:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --features mysql-db
```

Variáveis esperadas:

```bash
MANIFESTADOR_NFE_MYSQL_HOST=127.0.0.1
MANIFESTADOR_NFE_MYSQL_PORT=3306
MANIFESTADOR_NFE_MYSQL_DATABASE=MANIFESTADOR_NFE
MANIFESTADOR_NFE_MYSQL_USERNAME=root
MANIFESTADOR_NFE_MYSQL_PASSWORD=
```

## PostgreSQL

Compile com:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --features postgres-db
```

Variáveis esperadas:

```bash
MANIFESTADOR_NFE_POSTGRES_HOST=127.0.0.1
MANIFESTADOR_NFE_POSTGRES_PORT=5432
MANIFESTADOR_NFE_POSTGRES_DATABASE=MANIFESTADOR_NFE
MANIFESTADOR_NFE_POSTGRES_USERNAME=postgres
MANIFESTADOR_NFE_POSTGRES_PASSWORD=
```

## Validação local

```bash
npm install --no-audit --no-fund
npm run ci:version
npm run typecheck
npm run build:web
npm run fmt:rust
npm run fmt:rust:check
npm run lint:rust
npm run test:rust
npm run ci:rust:features
```

## Observação importante

O provider-based CRUD está iniciado para os cadastros genéricos. Módulos centrais como login, sessão, permissões, licenciamento e logs continuam preservando os fluxos existentes para evitar regressão brusca.
