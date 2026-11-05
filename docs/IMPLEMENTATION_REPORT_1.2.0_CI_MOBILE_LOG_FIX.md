# Relatório de correção — Template Tauri/Rust 1.2.0

## Escopo

Correções pontuais aplicadas após análise dos logs dos workflows CI, Android e iOS, sem alteração da arquitetura, remoção de recursos ou refatoração abrupta.

## Causas-raiz corrigidas

### Migração SQLite legada

O vetor de caminhos legados era criado sem tipo explícito. A inferência acabava convergindo para `Vec<Path>` e provocava os erros `E0277` e `E0599` no harness de migração e nos builds Android.

Correção: declaração explícita como `Vec<PathBuf>` em `src-tauri/src/legacy_data.rs`.

### Isolamento Cargo entre desktop e mobile

A feature `desktop` era default no Cargo. Como o build mobile não desabilitava as features default, Android e iOS herdavam tray, menu e APIs de janela exclusivas do desktop.

Correções:

- `default = []` no `Cargo.toml`;
- `tauri.conf.json` ativa `desktop` para builds Tauri normais;
- configurações Android/iOS continuam ativando somente `mobile`;
- comandos e workflows mobile passam `--features mobile` explicitamente;
- comandos de validação de bancos desktop passam suas features explicitamente.

### Recursos do frontend no iOS

O bundle iOS validava `src-tauri/dist/**/*` antes de encontrar os recursos preparados, gerando `glob pattern dist/**/* path not found`.

Correções:

- workflows Android/iOS usam `build:web:tauri`;
- o script iOS garante `dist/index.html` e executa `prepare-tauri-resources.mjs` antes de chamar o Tauri CLI.

### Fallback Xcode indevido

Qualquer menção a assinatura no log, inclusive um aviso não fatal, acionava o fallback `xcodebuild`. Isso mascarava falhas reais e levava ao erro de arquivo `server-addr` ausente.

Correção: o fallback só é executado quando há uma falha fatal de assinatura ou provisionamento reconhecida. Falhas de recursos, compilação ou configuração encerram preservando a causa original.

### Cota de artifacts no CI

O CI dependia do upload temporário de `dist`. Quando a cota de artifacts do GitHub era atingida, os jobs Rust e CloudPanel não podiam continuar.

Correções:

- upload/download temporário passou a ser uma otimização não bloqueante;
- jobs Rust criam assets mínimos determinísticos quando o artifact não está disponível;
- CloudPanel recompila o frontend localmente quando necessário;
- upload de pacote CloudPanel no CI geral é não bloqueante;
- publicação de artefatos no workflow de release permanece obrigatória e inalterada.

### Scripts de release ausentes nos logs

Os seis arquivos apontados como ausentes pelos logs estão presentes no template corrigido e são exigidos pelo validador estrutural:

- `scripts/release/collect-release-assets.mjs`;
- `scripts/release/upload-release-assets.mjs`;
- `scripts/release/write-build-status.mjs`;
- `scripts/release/enforce-release-policy.mjs`;
- `scripts/release/tests/release-pipeline.test.mjs`;
- `scripts/release/package-ios-unsigned.sh`.

## Validações executadas

- validador estrutural do projeto: aprovado;
- parsing de todos os JSON: aprovado;
- parsing dos workflows YAML: aprovado;
- sintaxe de todos os scripts Bash: aprovada;
- sintaxe de todos os scripts Node `.mjs`: aprovada;
- sintaxe Python do validador: aprovada;
- testes do pipeline de release: 9 de 9 aprovados;
- contrato de seleção desktop/mobile verificado estaticamente;
- arquivos obrigatórios de release verificados.

O ambiente de correção não possui `cargo`/`rustc`, Android SDK, Xcode ou CocoaPods. Portanto, a compilação nativa final de Rust, APK, AAB e IPA deve ser confirmada pelos próprios workflows incluídos no template.
