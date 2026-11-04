# Visual Core v12.1 - Correção Clippy Rust

## Correção aplicada

Corrigido erro do Clippy em `src-tauri/src/internal_api/routes.rs`:

```rust
async fn scalar_docs(State(_state): State<InternalApiState>, uri: Uri) -> impl IntoResponse
```

O parâmetro `state` era recebido pela assinatura da rota Axum, mas não era utilizado dentro da função `scalar_docs`, causando falha porque o projeto compila com `-D warnings`.

## Impacto

- Nenhuma regra de negócio foi alterada.
- Nenhuma rota foi removida.
- Nenhum payload foi alterado.
- Nenhum endpoint foi alterado.
- Apenas foi corrigido o warning tratado como erro pelo Clippy.

## Arquivo alterado

- `src-tauri/src/internal_api/routes.rs`
