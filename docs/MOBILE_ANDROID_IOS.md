# Mobile Tauri 2: Android e iOS

## Arquitetura preservada

O mobile usa o mesmo frontend Vue 3/TypeScript do desktop e do Web/PWA. O
conteúdo é empacotado localmente pelo Tauri; Android e iOS não carregam o
CloudPanel como WebView remoto.

Telas, componentes, dashboards, gráficos, filtros, formulários e regras de
apresentação executam no smartphone. O SQLite fica no diretório privado do
aplicativo. Conforme o produto derivado, o mobile pode operar de forma
independente ou consumir uma API REST do desktop, servidor ou CloudPanel e
persistir os dados localmente. Este template não implementa sincronização de um
domínio específico.

## Operação local ou remota

Na primeira abertura, o aplicativo mobile escolhe entre:

- **local**: SQLite no dispositivo, com login opcional e acesso local técnico quando o login estiver desativado;
- **remoto**: cliente autenticado da API do desktop/headless/CloudPanel, respeitando perfil, permissões e empresas vinculadas ao usuário.

A configuração remota usa `/health`, `/api/auth/*` e `/api/command`. O token global do gateway é opcional e fica somente na sessão atual. Consulte [`AUTHENTICATION_AND_MOBILE_ACCESS.md`](AUTHENTICATION_AND_MOBILE_ACCESS.md).


No runtime mobile nativo, rotas exclusivamente desktop (serviços locais, tray,
Scalar interno, webhooks, WebSocket e prévia em janela separada) são omitidas do
menu e bloqueadas também quando acessadas diretamente. A tela de banco permanece
disponível porque consulta o SQLite local do dispositivo.

A feature Cargo `desktop` permanece como padrão. A feature `mobile` ativa
Tauri/Wry para Android/iOS, sem tray, múltiplas janelas desktop, shell, serviços
Windows/Linux ou integrações gráficas exclusivas do desktop. O headless continua
compilando sem features padrão e sem Tauri/Wry/GTK.

## Estrutura

```text
src-tauri/tauri.android.conf.json
src-tauri/tauri.ios.conf.json
src-tauri/mobile/mobile.config.json
src-tauri/mobile/icons/
src-tauri/gen/android/
src-tauri/gen/apple/
```

Os diretórios `gen` são criados pelo Tauri CLI de modo idempotente. Os assets
mobile são derivados da identidade visual do próprio aplicativo. O gerador
valida fontes PNG 1024×1024, gera as variantes Android/iOS e restaura os ícones
desktop que já existiam; ele não lê nem modifica os ícones da habilidade.

## Android

Pré-requisitos do ambiente: Node 24, Java 17, Android SDK, Android NDK e Rust.
Targets suportados:

```text
aarch64-linux-android
armv7-linux-androideabi
x86_64-linux-android
i686-linux-android
```

Comandos:

```bash
npm run tauri -- android init
npm run tauri -- android dev
npm run tauri -- android build

npm run tauri:android:init
npm run tauri:android:dev
npm run tauri:android:build
npm run tauri:android:apk
npm run tauri:android:aab
```

O APK ARM64 de debug é instalável quando validado pelo `apksigner`. O AAB é
preparado para assinatura e publicação posterior. O coletor rejeita um AAB que
já contenha entradas de assinatura JAR em `META-INF`, para que um arquivo
nomeado `unsigned.aab` seja realmente não assinado. O template não inclui
keystore e não publica na Google Play.

Saídas normalizadas pela CI/release:

```text
release/android/<produto>-android-arm64-v<versao>-debug.apk
release/android/<produto>-android-arm64-v<versao>-debug.apk.sha256
release/android/<produto>-android-v<versao>-unsigned.aab
release/android/<produto>-android-v<versao>-unsigned.aab.sha256
```

## iOS e IPA não assinado

A compilação exige macOS com Xcode, CocoaPods e o target Rust `aarch64-apple-ios`.

```bash
npm run tauri -- ios init
npm run tauri -- ios dev
npm run tauri -- ios build

npm run tauri:ios:init
npm run tauri:ios:dev
npm run tauri:ios:build
npm run tauri:ios:unsigned
```

`scripts/mobile/build-ios-unsigned.sh` produz um bundle `.app` de dispositivo
físico ARM64 sem exigir conta Apple. O script limpa a saída temporária, marca o
início da execução e aceita somente bundles criados depois desse marcador,
evitando reutilizar um `.app` antigo. Em seguida:

```bash
bash scripts/release/package-ios-unsigned.sh \
  /caminho/Produto.app \
  release/ios/<produto>-ios-arm64-v<versao>-unsigned.ipa
```

O empacotador valida:

- existência do `.app` e do `Info.plist`;
- `CFBundleExecutable` e permissão de execução;
- Mach-O ARM64 por `file` e `lipo`;
- plataforma física `IOS`, rejeitando `IOSSIMULATOR`;
- ausência de provisioning profile e de qualquer assinatura, inclusive ad-hoc;
- estrutura real `Payload/Produto.app/` no ZIP;
- integridade do IPA e SHA-256.

Saída:

```text
release/ios/<produto>-ios-arm64-v<versao>-unsigned.ipa
release/ios/<produto>-ios-arm64-v<versao>-unsigned.ipa.sha256
```

O IPA precisa ser assinado posteriormente pelo proprietário da conta Apple
Developer. O template não inclui Apple ID, certificado, `.p12`, provisioning
profile, App Store Connect API Key, TestFlight ou publicação na App Store.

## Workflows

```text
.github/workflows/mobile-android.yml
.github/workflows/mobile-ios.yml
.github/workflows/release.yml
```

Os workflows mobile usam `npm ci`, Node 24, lockfile Cargo,
`actions/upload-artifact@v7`, geram checksums e registram os estados:

```text
android-arm64-apk
android-aab
ios-arm64-unsigned-ipa
```

Esses estados integram a mesma release coordenada, sem criar release paralela.
