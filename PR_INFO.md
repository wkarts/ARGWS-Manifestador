# PR — autenticação híbrida desktop/mobile e acesso por perfil

## Branch

`feat/mobile-android-ios-tauri-1-2-1`

## Título

`feat(auth): adiciona bootstrap interno, senha configurável e acesso mobile híbrido`

## Versão

`1.2.1`

A promoção para 1.2.1 incorpora também as correções dos logs da PR #5:

- corrige `E0597` no carregamento SQLite de permissões;
- remove a dependência `uuid` do harness e reutiliza `rand` para gerar o segredo interno do usuário mobile local;
- restaura o `Cargo.lock` conhecido do harness e mantém sua execução com `--locked`.
- aplica os 21 hunks de formatação exigidos por `cargo fmt --check` no Rust 1.97.1, sem mudança funcional.

## Descrição

Evolui a template Tauri/Rust 1.2.1 para oferecer primeiro acesso administrativo
seguro, política de senha configurável e operação mobile híbrida, preservando os
recursos desktop, Web/PWA, headless, CloudPanel, Android e iOS já existentes.

O aplicativo mobile pode funcionar de forma independente, com SQLite privado no
dispositivo e login opcional, ou como cliente autenticado da API publicada pelo
desktop, servidor headless ou CloudPanel.

A implementação foi realizada de forma incremental, sem refatoração arquitetural
abrupta e sem remover contratos existentes.

## Primeiro acesso administrativo

- Remove a criação de novos arquivos com senha temporária.
- Mantém a credencial inicial exclusivamente no backend Rust.
- Permite definir a primeira senha diretamente na aplicação desktop.
- Permite primeiro acesso no mobile local quando o produto exigir autenticação.
- Suporta usuário e senha iniciais por variáveis de ambiente em instalações
  headless.
- Trata a senha definida por ambiente como definitiva.
- Remove arquivos `.bootstrap-admin.local` deixados por versões anteriores.
- Impede que senha de bootstrap seja enviada ao frontend, gravada em configuração
  pública ou incluída em documentação gerada.

## Compatibilidade de migração administrativa

- Localiza primeiro um master existente.
- Na ausência de master, promove o login legado `admin` ou o primeiro usuário administrador.
- Preserva o hash de senha do administrador legado durante a promoção.
- Aplica `MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD` somente quando o master existente ainda está com senha provisória.
- Impede colisão de login e duplicação do usuário administrador em bancos atualizados.
- Mantém a limpeza idempotente do arquivo bootstrap legado.

## Política de senha

- Centraliza validação e persistência no backend Rust.
- Aplica a mesma política no primeiro acesso, troca de senha e manutenção de
  usuários.
- Usa mínimo padrão de 8 e máximo de 128 caracteres.
- Exige letra e número por padrão.
- Mantém maiúscula, minúscula e símbolo opcionais por padrão.
- Permite ao master parametrizar cada requisito.
- Mantém limite técnico mínimo configurável de 6 caracteres.

## Mobile local independente

- Usa SQLite privado no dispositivo.
- Permite acesso sem tela de login por padrão.
- Permite ativar login local quando a aplicação derivada exigir.
- Cria sessão técnica exclusiva do canal `mobile-local`.
- Não expõe o comando de acesso sem login no runtime desktop.
- Não inicia tray, Scalar, API HTTP, Webhook Service, WebSocket Service ou
  serviços do sistema no runtime mobile.
- Mantém dashboards, formulários, gráficos e persistência no próprio aplicativo.

## Mobile como cliente remoto

- Configura nome da conexão, URL da API e header do token global.
- Autentica em `POST /api/auth/login` usando o canal `mobile`.
- Aceita pela rede somente os canais `api` e `mobile`; o canal desktop é recusado pelo endpoint HTTP.
- Restaura e encerra sessões por endpoints específicos.
- Executa comandos em `POST /api/command`.
- Envia automaticamente o `session_token` do usuário.
- Envia a empresa ativa quando aplicável.
- Mantém o token global do gateway apenas em `sessionStorage`.
- Invalida a sessão anterior antes de trocar entre modo local e remoto.

## Acesso por usuário, perfil e empresa

Adiciona aos usuários:

- `api_access_enabled`;
- `mobile_access_enabled`;
- `api_session_timeout_minutes`;
- perfis autorizados;
- empresas autorizadas.

Adiciona as permissões:

- `api:access`;
- `mobile:access`.

A API valida em cada comando protegido:

1. validade e canal da sessão;
2. habilitação individual para API;
3. habilitação individual para mobile;
4. permissões dos perfis vinculados;
5. empresas vinculadas ao usuário;
6. conclusão da troca de senha provisória.

## Segurança administrativa da API

- Restringe alteração de usuários, perfis e políticas ao usuário master.
- Impede administração do usuário técnico local.
- Impede que o master da sessão se inative ou remova o próprio perfil master.
- Impede exclusão de usuários master e perfis ainda vinculados.
- Remove hashes de senha das respostas.
- Valida login duplicado, e-mail, senha, perfis e empresas antes de salvar.
- Preserva vínculos existentes quando uma atualização parcial não os informa.
- Documenta autenticação e comandos no OpenAPI/Scalar.

## Escopo por empresa

- Filtra listagens e combos de empresas pelos vínculos do usuário.
- Bloqueia leitura, alteração e exclusão fora do escopo.
- Vincula automaticamente uma empresa criada ao usuário não master que a criou.
- Mantém acesso geral para o usuário master.
- Mantém a empresa ativa no estado da sessão do frontend.

As entidades genéricas não recebem `empresa_id` automaticamente. Cada aplicação
derivada deve declarar explicitamente quais tabelas são multiempresa e aplicar o
mesmo contrato em seus repositórios, evitando alteração indevida de modelos
existentes.

## Compatibilidade preservada

- Desktop Tauri 2.
- Web/PWA.
- API headless e CloudPanel.
- CLI e worker.
- SQLite, MySQL/MariaDB e PostgreSQL.
- Migração SQLite legada.
- Android APK e AAB.
- iOS ARM64 e IPA não assinado.
- Checksums, manifesto e status de release.
- Licenciamento, integrações, logs e bootstrap existentes.

## Validação

```bash
python3 scripts/ci/validate-tauri-project.py --project . --mode static
node --test scripts/release/tests/*.test.mjs
find scripts deploy -type f -name '*.sh' -print0 | xargs -0 -n1 bash -n
find scripts -type f -name '*.mjs' -print0 | xargs -0 -n1 node --check
python3 -m py_compile scripts/ci/validate-tauri-project.py
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo check --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features
```

Os builds Android exigem Android SDK/NDK. O build iOS exige macOS, Xcode e
CocoaPods.

## Riscos e rollback

As alterações ficam isoladas na autenticação, autorização, configuração mobile,
migrations incrementais e endpoints da API.

O principal risco é uma aplicação derivada consumir entidades multiempresa sem
adicionar o respectivo campo e filtro de empresa. Esse ponto permanece explícito
e não é automatizado pela template.

O rollback consiste em reverter este PR e restaurar a revisão R3. Nenhum
artefato, plataforma ou nome existente foi removido.

## Commit principal

`feat(auth): adiciona bootstrap interno, senha configurável e acesso mobile híbrido`

## Commit desta correção

`fix(ci): aplica rustfmt da 1.2.1 e estabiliza harness de migração`
