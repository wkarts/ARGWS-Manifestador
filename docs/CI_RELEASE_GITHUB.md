# CI/CD e release no GitHub

O template possui workflows complementares de CI, release, CloudPanel, Android e iOS em `.github/workflows`.

## CI

`ci.yml` executa em Pull Requests, `main`, `develop`, `release/**` e por
acionamento manual. O gate inclui:

- `actionlint` e validação de título de Pull Request;
- auditoria estrutural do projeto;
- sintaxe dos scripts Node.js, shell e PowerShell;
- `npm ci`, sincronização de versão, TypeScript e build Web/PWA;
- `cargo fmt`, `cargo check`, `cargo clippy` e testes com `--locked`;
- matriz das features MySQL, PostgreSQL e combinadas;
- build headless puro com `--no-default-features`;
- sintaxe dos scripts de empacotamento iOS e contrato mobile estático.

## Release coordenada

`release.yml` inicia somente depois de um CI aprovado em `main`, ou manualmente
para retomar uma tag já existente. O fluxo:

1. resolve a próxima versão com `semantic-release`;
2. gera e versiona `src-tauri/Cargo.lock` antes da tag;
3. faz checkout da tag exata em todos os builds;
4. gera Windows x64/x86, Linux x64/ARM64, macOS Intel/Apple Silicon,
   Web/PWA, CloudPanel x64/x86, Android APK/AAB e iOS ARM64 IPA não assinado;
5. reúne os artefatos em um único job publicador;
6. renomeia os arquivos e gera `SHA256SUMS.txt` e
   `RELEASE-MANIFEST.json`;
7. cria ou reutiliza um único draft; uma matriz completa é publicada e uma
   matriz parcial permanece retomável por padrão.

Uma publicação parcial exige acionamento manual com `allow_partial_release`.
Depois dessa autorização, alvos ausentes só podem ser acrescentados em uma nova
versão, pois a release publicada não é reaberta.

Uma nova tentativa aceita asset remoto existente somente quando o conteúdo é
idêntico. Release publicada não é reaberta, tag não é movida e asset divergente
não é sobrescrito.

## Mobile

`mobile-android.yml` configura Java, Android SDK/NDK e os quatro targets Rust
Android somente no runner Linux destinado ao mobile. Ele gera APK ARM64 de
debug e AAB, localiza os arquivos reais do Gradle/Tauri, valida-os e publica os
checksums.

`mobile-ios.yml` executa em macOS com Xcode, CocoaPods e `aarch64-apple-ios`, gera o `.app`
para dispositivo físico e cria um IPA real com `Payload/<produto>.app`. A
ausência de assinatura não é erro. O IPA é identificado como `unsigned` e deve
ser assinado posteriormente. Nenhuma credencial Apple/Google ou publicação em
loja está configurada.

Os estados `android-arm64-apk`, `android-aab` e
`ios-arm64-unsigned-ipa` ampliam a matriz coordenada de nove para doze alvos.

## CloudPanel manual

`cloudpanel-linux-release.yml` gera, sob demanda, pacotes x64, x86 ou ambos e os
mantém como artefatos da execução. A publicação oficial desses pacotes ocorre
no release coordenado, evitando duas rotas concorrentes gravando na mesma
release.

## Windows e caminhos longos

Os builds locais e o release Windows usam `CARGO_TARGET_DIR` curto fora do
projeto para evitar `LNK1104`, `MAX_PATH` e falhas em diretórios com espaços.
Use `npm run build:windows:project-target` apenas quando precisar
explicitamente do target dentro de `src-tauri`.

## Segredos opcionais

- `WINDOWS_CERTIFICATE`: certificado PFX codificado em Base64.
- `WINDOWS_CERTIFICATE_PASSWORD`: senha do PFX.

Sem esses secrets, os instaladores são gerados sem assinatura. O workflow
remove o arquivo temporário e o certificado importado ao concluir o job.
