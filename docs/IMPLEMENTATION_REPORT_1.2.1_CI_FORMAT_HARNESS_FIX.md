# Template 1.2.1 — correção CI rustfmt e harness SQLite

## Execução analisada

Pacote de logs: `logs_84539664003.zip`.

## Falhas encontradas

1. `Rust and Tauri / Check formatting` terminou com exit code 1.
   O Rust 1.97.1 reportou 21 hunks de `rustfmt` em quatro arquivos:
   - `src-tauri/src/commands/auth.rs`;
   - `src-tauri/src/internal_api/app_commands.rs`;
   - `src-tauri/src/internal_api/routes.rs`;
   - `src-tauri/src/migrations.rs`.
2. `Legacy SQLite migration and bootstrap` terminou com exit code 101 porque
   `cargo run --locked` detectou divergência entre `Cargo.toml` e `Cargo.lock`.
   A dependência `uuid` havia sido adicionada ao harness apenas porque `migrations.rs`
   gerava o segredo interno do usuário mobile local com `Uuid::new_v4()`.

## Correção

- Aplicados literalmente os 21 hunks emitidos por `cargo fmt --check`.
- A geração do segredo interno do usuário mobile local passa a usar um `u128`
  aleatório via `rand`, dependência que já fazia parte do aplicativo e do harness.
- Removida `uuid` do `tests/legacy-migration-harness/Cargo.toml`.
- Restaurado o `Cargo.lock` conhecido do harness, idêntico ao grafo anterior à
  introdução de `uuid`, preservando `cargo run --locked`.
- Adicionadas verificações de regressão para impedir que `migrations.rs` volte a
  exigir `uuid` no harness sem atualização consistente do lockfile.
- Mantida a versão 1.2.1 em todos os contratos de versionamento.

A alteração do segredo é interna e não é exposta ao frontend ou ao usuário. O
valor continua sendo usado apenas como entrada para o hash da conta técnica local.

## Validações locais

- sincronismo de versão 1.2.1;
- validador estrutural;
- 10/10 testes do pipeline de release;
- sintaxe Node/Bash/Python;
- JSON;
- paridade exata entre dependências do `Cargo.toml` do harness e a lista raiz do
  `Cargo.lock`;
- ausência de `uuid` no grafo do harness;
- `git diff --check`.

## Observações não bloqueantes

O frontend apresentou avisos do Vite sobre módulos importados simultaneamente de
forma estática e dinâmica e uma disputa de gravação do cache npm. O build Web/PWA
foi concluído com sucesso; esses avisos não causaram a falha da execução e não
foram tratados como erro funcional nesta correção.
