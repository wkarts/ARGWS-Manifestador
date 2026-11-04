# Relatório de implementação — Template Tauri/Rust 1.2.1

## Objetivo

Promover a revisão de autenticação/mobile híbrido para 1.2.1 e corrigir as regressões identificadas nos logs da PR #5 sem refatoração abrupta.

## Correções

- Sincronização da versão 1.2.1 em VERSION, Node, Rust/Tauri, configuração do frontend, mobile e CloudPanel.
- Correção de `E0597` em `src-tauri/src/commands/auth.rs`, materializando o vetor de permissões antes do fim do escopo do statement SQLite.
- Sincronização do lockfile do harness de migração com a dependência `uuid` já declarada em seu Cargo.toml.
- Preservação de `cargo run --manifest-path tests/legacy-migration-harness/Cargo.toml --locked`.

## Compatibilidade

Nenhuma regra de negócio, endpoint, formato de banco, artefato mobile ou arquitetura desktop/headless foi removida.
