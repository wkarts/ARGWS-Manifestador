import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');

function temporaryDirectory(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function writeJson(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

test('prepare-release sincroniza e valida o Cargo.lock antes da tag', () => {
  const fixture = temporaryDirectory('template-prepare-release-');
  const cargoLog = path.join(fixture, 'cargo.log');
  const fakeCargo = path.join(fixture, 'fake-cargo.sh');
  fs.writeFileSync(fakeCargo, '#!/usr/bin/env bash\nprintf "%s\\n" "$*" >> "${FAKE_CARGO_LOG}"\n', 'utf8');
  fs.chmodSync(fakeCargo, 0o755);

  fs.mkdirSync(path.join(fixture, 'src-tauri'), { recursive: true });
  fs.mkdirSync(path.join(fixture, 'src', 'config'), { recursive: true });
  fs.writeFileSync(path.join(fixture, 'VERSION'), '1.0.0\n');
  writeJson(path.join(fixture, 'package.json'), { name: 'manifestador-nfe', version: '1.0.0' });
  writeJson(path.join(fixture, 'package-lock.json'), {
    name: 'manifestador-nfe',
    version: '1.0.0',
    packages: { '': { name: 'manifestador-nfe', version: '1.0.0' } },
  });
  writeJson(path.join(fixture, 'src-tauri', 'tauri.conf.json'), { productName: 'Manifestador NF-e', version: '1.0.0' });
  fs.writeFileSync(
    path.join(fixture, 'src-tauri', 'Cargo.toml'),
    '[package]\nname = "manifestador_nfe"\nversion = "1.0.0"\nedition = "2021"\n',
  );
  fs.writeFileSync(
    path.join(fixture, 'src-tauri', 'Cargo.lock'),
    'version = 4\n\n[[package]]\nname = "manifestador_nfe"\nversion = "1.0.0"\n',
  );
  fs.writeFileSync(
    path.join(fixture, 'src', 'config', 'projectConfig.ts'),
    'export const projectConfig = { version: "1.0.0" };\n',
  );

  const result = spawnSync(
    process.execPath,
    [path.join(repositoryRoot, 'scripts', 'release', 'prepare-release.mjs'), '2.0.0'],
    {
      cwd: fixture,
      encoding: 'utf8',
      env: { ...process.env, CARGO: fakeCargo, FAKE_CARGO_LOG: cargoLog },
    },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.match(fs.readFileSync(path.join(fixture, 'src-tauri', 'Cargo.lock'), 'utf8'), /version = "2\.0\.0"/);
  assert.equal(JSON.parse(fs.readFileSync(path.join(fixture, 'package.json'))).version, '2.0.0');
  assert.match(fs.readFileSync(cargoLog, 'utf8'), /metadata --manifest-path src-tauri\/Cargo\.toml --locked --format-version 1/);
});

test('coletor publica artefatos aprovados e registra matriz parcial', () => {
  const fixture = temporaryDirectory('template-collect-release-');
  const input = path.join(fixture, 'artifacts');
  const output = path.join(fixture, 'release-assets');
  fs.mkdirSync(path.join(input, 'release-desktop-windows-x64'), { recursive: true });
  fs.writeFileSync(path.join(input, 'release-desktop-windows-x64', 'ManifestadorNFe.exe'), 'binary');

  writeJson(
    path.join(input, 'release-status-desktop-windows-x64', 'desktop-windows-x64.status.json'),
    {
      id: 'desktop-windows-x64',
      label: 'Windows x64',
      status: 'success',
      artifact: 'release-desktop-windows-x64',
      runUrl: 'https://github.example/actions/runs/1',
    },
  );
  writeJson(
    path.join(input, 'release-status-cloudpanel-linux-x86', 'cloudpanel-linux-x86.status.json'),
    {
      id: 'cloudpanel-linux-x86',
      label: 'CloudPanel Linux x86',
      status: 'failure',
      artifact: 'release-cloudpanel-linux-x86',
      runUrl: 'https://github.example/actions/runs/1',
      error: 'Dependência i386 indisponível.',
    },
  );
  writeJson(path.join(fixture, 'package.json'), { name: 'manifestador-nfe' });
  fs.writeFileSync(path.join(fixture, 'VERSION'), '1.24.1\n');

  const result = spawnSync(
    process.execPath,
    [
      path.join(repositoryRoot, 'scripts', 'release', 'collect-release-assets.mjs'),
      '--input', input,
      '--output', output,
      '--version', '1.24.1',
    ],
    { cwd: fixture, encoding: 'utf8' },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const manifest = JSON.parse(fs.readFileSync(path.join(output, 'RELEASE-MANIFEST.json')));
  assert.equal(manifest.assets.length, 1);
  assert.equal(manifest.buildMatrix.outcome, 'partial');
  assert.equal(manifest.buildMatrix.succeeded, 1);
  assert.equal(manifest.buildMatrix.failed, 11);
  const report = fs.readFileSync(path.join(output, 'RELEASE-STATUS.md'), 'utf8');
  assert.match(report, /Matriz parcial/);
  assert.match(report, /CloudPanel Linux x86/);
  assert.match(report, /Dependência i386 indisponível/);
});

test('política mantém matriz parcial em draft sem autorização explícita', () => {
  const fixture = temporaryDirectory('template-release-policy-draft-');
  const status = path.join(fixture, 'status.json');
  const output = path.join(fixture, 'github-output.txt');
  writeJson(status, { outcome: 'partial', total: 12, succeeded: 11, failed: 1 });

  const result = spawnSync(
    process.execPath,
    [
      path.join(repositoryRoot, 'scripts', 'release', 'enforce-release-policy.mjs'),
      '--status', status,
      '--allow-partial', 'false',
      '--output', output,
    ],
    { encoding: 'utf8' },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const decisions = fs.readFileSync(output, 'utf8');
  assert.match(decisions, /publish=false/);
  assert.match(decisions, /decision=partial-draft/);
});

test('política publica matriz completa ou parcial explicitamente autorizada', () => {
  for (const fixtureCase of [
    { outcome: 'complete', allow: 'false', decision: 'complete' },
    { outcome: 'partial', allow: 'true', decision: 'partial-authorized' },
  ]) {
    const fixture = temporaryDirectory('template-release-policy-publish-');
    const status = path.join(fixture, 'status.json');
    const output = path.join(fixture, 'github-output.txt');
    writeJson(status, {
      outcome: fixtureCase.outcome,
      total: 12,
      succeeded: fixtureCase.outcome === 'complete' ? 12 : 11,
      failed: fixtureCase.outcome === 'complete' ? 0 : 1,
    });
    const result = spawnSync(
      process.execPath,
      [
        path.join(repositoryRoot, 'scripts', 'release', 'enforce-release-policy.mjs'),
        '--status', status,
        '--allow-partial', fixtureCase.allow,
        '--output', output,
      ],
      { encoding: 'utf8' },
    );
    assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
    const decisions = fs.readFileSync(output, 'utf8');
    assert.match(decisions, /publish=true/);
    assert.match(decisions, new RegExp(`decision=${fixtureCase.decision}`));
  }
});

test('gravador de status preserva falha e URL da execução', () => {
  const fixture = temporaryDirectory('template-build-status-');
  const output = path.join(fixture, 'cloudpanel-linux-x86.status.json');
  const result = spawnSync(
    process.execPath,
    [
      path.join(repositoryRoot, 'scripts', 'release', 'write-build-status.mjs'),
      '--output', output,
      '--id', 'cloudpanel-linux-x86',
      '--label', 'CloudPanel Linux x86',
      '--platform', 'cloudpanel-linux',
      '--arch', 'x86',
      '--target', 'i686-unknown-linux-gnu',
      '--artifact', 'release-cloudpanel-linux-x86',
    ],
    {
      encoding: 'utf8',
      env: {
        ...process.env,
        BUILD_STATUS: 'failure',
        RELEASE_RUN_URL: 'https://github.example/actions/runs/2',
      },
    },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const status = JSON.parse(fs.readFileSync(output));
  assert.equal(status.success, false);
  assert.equal(status.status, 'failure');
  assert.match(status.error, /actions\/runs\/2/);
});

test('CloudPanel isola Tauri/GTK e compila x64 e x86 no CI', () => {
  const buildScript = fs.readFileSync(
    path.join(repositoryRoot, 'scripts', 'linux', 'build-cloudpanel-release.sh'),
    'utf8',
  );
  const dependencyScript = fs.readFileSync(
    path.join(repositoryRoot, 'scripts', 'linux', 'install-cloudpanel-build-deps.sh'),
    'utf8',
  );
  const cargoManifest = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'Cargo.toml'),
    'utf8',
  );
  const buildSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'build.rs'),
    'utf8',
  );
  const librarySource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'lib.rs'),
    'utf8',
  );
  const authCommands = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'commands', 'auth.rs'),
    'utf8',
  );
  const entityCommands = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'commands', 'entities.rs'),
    'utf8',
  );
  const ciWorkflow = fs.readFileSync(
    path.join(repositoryRoot, '.github', 'workflows', 'ci.yml'),
    'utf8',
  );

  assert.match(cargoManifest, /desktop = \["dep:tauri", "dep:tauri-build"/);
  assert.match(cargoManifest, /tauri = \{[^}]*optional = true[^}]*\}/);
  assert.match(cargoManifest, /tauri-build = \{[^}]*optional = true[^}]*\}/);
  assert.match(buildSource, /#\[cfg\(any\(feature = "desktop", feature = "mobile"\)\)\]\n    tauri_build::build\(\);/);
  assert.match(buildScript, /--no-default-features/);
  assert.match(buildScript, /cargo tree/);
  assert.match(buildScript, /--edges normal,build/);
  assert.match(buildScript, /forbidden_headless_crates/);
  assert.match(buildScript, /tauri-build/);
  assert.match(buildScript, /tauri-runtime-wry/);
  assert.match(buildScript, /pkg-config --exists openssl/);
  assert.match(buildScript, /CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_RUSTFLAGS=-C link-arg=-m32/);
  assert.doesNotMatch(buildScript, /npm install/);
  assert.doesNotMatch(buildScript, /prepare-tauri-resources\.mjs \|\| true/);
  assert.match(librarySource, /#\[cfg\(any\(feature = "desktop", feature = "mobile"\)\)\]\n    pub mod access;/);
  assert.doesNotMatch(librarySource, /#\[cfg\(feature = "desktop"\)\]\n    pub mod auth;/);
  assert.doesNotMatch(librarySource, /#\[cfg\(feature = "desktop"\)\]\n    pub mod entities;/);
  assert.match(authCommands, /#\[cfg\(any\(feature = "desktop", feature = "mobile"\)\)\]\n#\[tauri::command\]\npub fn auth_login/);
  assert.match(entityCommands, /#\[cfg\(any\(feature = "desktop", feature = "mobile"\)\)\]\n#\[tauri::command\]\npub fn entity_list/);
  assert.match(ciWorkflow, /cloudpanel-headless:/);
  assert.match(ciWorkflow, /Build and package CloudPanel headless/);
  assert.match(ciWorkflow, /BUILD_WEB: "false"/);
  assert.doesNotMatch(dependencyScript, /libayatana-appindicator3-dev:i386/);
  assert.doesNotMatch(dependencyScript, /libxdo-dev:i386/);
  assert.doesNotMatch(dependencyScript, /libwebkit2gtk-4\.1-dev:i386/);
});


test('coletor inclui APK, AAB e IPA na matriz coordenada de doze alvos', () => {
  const fixture = temporaryDirectory('template-collect-mobile-release-');
  const input = path.join(fixture, 'artifacts');
  const output = path.join(fixture, 'release-assets');
  const mobileAssets = [
    {
      id: 'android-arm64-apk',
      label: 'Android APK ARM64',
      artifact: 'release-android-arm64-apk',
      file: 'produto-android-arm64-v1.2.0-debug.apk',
    },
    {
      id: 'android-aab',
      label: 'Android AAB',
      artifact: 'release-android-aab',
      file: 'produto-android-v1.2.0-unsigned.aab',
    },
    {
      id: 'ios-arm64-unsigned-ipa',
      label: 'iOS ARM64 IPA não assinado',
      artifact: 'release-ios-arm64-unsigned-ipa',
      file: 'produto-ios-arm64-v1.2.0-unsigned.ipa',
    },
  ];
  for (const target of mobileAssets) {
    fs.mkdirSync(path.join(input, target.artifact), { recursive: true });
    fs.writeFileSync(path.join(input, target.artifact, target.file), 'mobile-binary');
    writeJson(
      path.join(input, `release-status-${target.id}`, `${target.id}.status.json`),
      {
        id: target.id,
        label: target.label,
        status: 'success',
        artifact: target.artifact,
      },
    );
  }
  writeJson(path.join(fixture, 'package.json'), { name: 'produto' });
  fs.writeFileSync(path.join(fixture, 'VERSION'), '1.2.0\n');

  const result = spawnSync(
    process.execPath,
    [
      path.join(repositoryRoot, 'scripts', 'release', 'collect-release-assets.mjs'),
      '--input', input,
      '--output', output,
      '--version', '1.2.0',
    ],
    { cwd: fixture, encoding: 'utf8' },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const manifest = JSON.parse(fs.readFileSync(path.join(output, 'RELEASE-MANIFEST.json')));
  assert.equal(manifest.assets.length, 3);
  assert.equal(manifest.buildMatrix.total, 12);
  assert.equal(manifest.buildMatrix.succeeded, 3);
  assert.equal(manifest.buildMatrix.failed, 9);
  for (const target of mobileAssets) {
    assert.equal(manifest.buildMatrix.targets.find((item) => item.id === target.id)?.success, true);
  }
});

test('gitignore preserva scripts de entrega e ignora somente a saída de release', () => {
  const gitignore = fs.readFileSync(path.join(repositoryRoot, '.gitignore'), 'utf8');
  const requiredDeliveryFiles = [
    'collect-release-assets.mjs',
    'upload-release-assets.mjs',
    'write-build-status.mjs',
    'enforce-release-policy.mjs',
    'package-ios-unsigned.sh',
  ];

  assert.match(gitignore, /^\/release\/$/m);
  assert.doesNotMatch(gitignore, /^release\/$/m);
  for (const file of requiredDeliveryFiles) {
    assert.equal(fs.existsSync(path.join(repositoryRoot, 'scripts', 'release', file)), true);
  }
  assert.equal(
    fs.existsSync(path.join(repositoryRoot, 'scripts', 'release', 'tests', 'release-pipeline.test.mjs')),
    true,
  );
});

test('contrato mobile mantém frontend local, features isoladas e IPA real sem credenciais', () => {
  const packageJson = JSON.parse(fs.readFileSync(path.join(repositoryRoot, 'package.json')));
  const cargoManifest = fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'Cargo.toml'), 'utf8');
  const buildSource = fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'build.rs'), 'utf8');
  const librarySource = fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8');
  const tauriConfig = JSON.parse(fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'tauri.conf.json')));
  const androidConfig = JSON.parse(fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'tauri.android.conf.json')));
  const iosConfig = JSON.parse(fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'tauri.ios.conf.json')));
  const mobileConfig = JSON.parse(fs.readFileSync(path.join(repositoryRoot, 'src-tauri', 'mobile', 'mobile.config.json')));
  const mobileInitializer = fs.readFileSync(path.join(repositoryRoot, 'scripts', 'mobile', 'init-mobile.mjs'), 'utf8');
  const androidWorkflow = fs.readFileSync(path.join(repositoryRoot, '.github', 'workflows', 'mobile-android.yml'), 'utf8');
  const iosWorkflow = fs.readFileSync(path.join(repositoryRoot, '.github', 'workflows', 'mobile-ios.yml'), 'utf8');
  const releaseWorkflow = fs.readFileSync(path.join(repositoryRoot, '.github', 'workflows', 'release.yml'), 'utf8');
  const ipaScript = fs.readFileSync(path.join(repositoryRoot, 'scripts', 'release', 'package-ios-unsigned.sh'), 'utf8');
  const androidArtifactCollector = fs.readFileSync(path.join(repositoryRoot, 'scripts', 'mobile', 'collect-android-artifact.mjs'), 'utf8');
  const iosUnsignedBuilder = fs.readFileSync(path.join(repositoryRoot, 'scripts', 'mobile', 'build-ios-unsigned.sh'), 'utf8');
  const collector = fs.readFileSync(path.join(repositoryRoot, 'scripts', 'release', 'collect-release-assets.mjs'), 'utf8');
  const routerSource = fs.readFileSync(path.join(repositoryRoot, 'src', 'router', 'index.ts'), 'utf8');
  const navigationSource = fs.readFileSync(path.join(repositoryRoot, 'src', 'config', 'navigation.ts'), 'utf8');

  assert.match(cargoManifest, /default = \[\]/);
  assert.deepEqual(tauriConfig.build.features, ['desktop']);
  assert.match(cargoManifest, /mobile = \["dep:tauri", "dep:tauri-build", "tauri\/wry", "tauri\/custom-protocol"\]/);
  const mobileFeature = cargoManifest.match(/^mobile = \[(.*)\]$/m)?.[1] ?? '';
  assert.match(mobileFeature, /tauri\/wry/);
  assert.doesNotMatch(mobileFeature, /tray-icon|image-png/);
  assert.match(buildSource, /feature = "mobile"/);
  assert.match(librarySource, /#\[cfg\(all\(any\(target_os = "android", target_os = "ios"\), feature = "desktop"\)\)\]/);
  assert.match(librarySource, /compile_error!/);
  assert.match(librarySource, /#\[cfg_attr\(mobile, tauri::mobile_entry_point\)\]/);
  assert.match(librarySource, /app_local_data_dir\(\)/);
  assert.deepEqual(androidConfig.build.features, ['mobile']);
  assert.deepEqual(iosConfig.build.features, ['mobile']);
  assert.equal(mobileConfig.frontend.strategy, 'local-bundled');
  assert.equal(mobileConfig.frontend.remoteWebView, false);
  assert.equal(mobileConfig.frontend.localDatabase, 'sqlite');
  assert.equal(mobileConfig.android.primaryRustTarget, 'aarch64-linux-android');
  assert.equal(mobileConfig.ios.primaryRustTarget, 'aarch64-apple-ios');

  for (const script of [
    'tauri:android:init', 'tauri:android:dev', 'tauri:android:build',
    'tauri:ios:init', 'tauri:ios:dev', 'tauri:ios:build',
  ]) {
    assert.equal(typeof packageJson.scripts[script], 'string');
  }
  for (const script of [
    'tauri:android:dev', 'tauri:android:build',
    'tauri:ios:dev', 'tauri:ios:build',
  ]) {
    assert.match(packageJson.scripts[script], /--features mobile/);
  }
  assert.match(mobileInitializer, /--skip-targets-install/);
  assert.match(androidWorkflow, /aarch64-linux-android/);
  assert.match(androidWorkflow, /--apk --debug --ci/);
  assert.match(androidWorkflow, /--aab --ci/);
  assert.match(androidWorkflow, /GITHUB_PATH/);
  assert.match(androidWorkflow, /build-tools\/\$\{ANDROID_BUILD_TOOLS\}/);
  assert.match(androidWorkflow, /actions\/upload-artifact@v7/);
  assert.match(androidWorkflow, /Store Android artifact outside pull requests/);
  assert.match(androidWorkflow, /if: github\.event_name != 'pull_request'/);
  assert.match(iosWorkflow, /runs-on: macos-15/);
  assert.match(iosWorkflow, /aarch64-apple-ios/);
  assert.match(iosWorkflow, /Validate Xcode and CocoaPods/);
  assert.match(iosWorkflow, /brew install cocoapods/);
  assert.match(iosWorkflow, /pod --version/);
  assert.match(iosWorkflow, /package-ios-unsigned\.sh/);
  assert.match(iosWorkflow, /Store unsigned iOS IPA outside pull requests/);
  assert.match(iosWorkflow, /if: github\.event_name != 'pull_request'/);
  assert.match(releaseWorkflow, /^  android:/m);
  assert.match(releaseWorkflow, /^  ios:/m);
  assert.match(releaseWorkflow, /release-android-arm64-apk/);
  assert.match(releaseWorkflow, /release-android-aab/);
  assert.match(releaseWorkflow, /release-ios-arm64-unsigned-ipa/);
  assert.match(releaseWorkflow, /Validate Xcode and CocoaPods/);
  assert.match(releaseWorkflow, /brew install cocoapods/);
  assert.match(releaseWorkflow, /pod --version/);
  assert.match(routerSource, /Rota exclusiva do runtime desktop bloqueada no mobile nativo/);
  assert.match(routerSource, /desktopOnly: true/);
  assert.match(navigationSource, /Parâmetros.*, desktopOnly: true/);
  assert.match(ipaScript, /Payload/);
  assert.match(ipaScript, /CFBundleExecutable/);
  assert.match(ipaScript, /platform IOSSIMULATOR/);
  assert.match(ipaScript, /shasum -a 256/);
  assert.match(androidArtifactCollector, /isJarSignatureEntry/);
  assert.match(androidArtifactCollector, /META-INF/);
  assert.match(androidArtifactCollector, /AAB contém assinatura JAR inesperada/);
  assert.match(iosUnsignedBuilder, /\.build-started-at/);
  assert.match(iosUnsignedBuilder, /-newer/);
  assert.match(iosUnsignedBuilder, /rm -rf "\$\{GEN_DIR\}\/build"/);
  assert.match(iosUnsignedBuilder, /prepare-tauri-resources\.mjs/);
  assert.match(iosUnsignedBuilder, /--features mobile/);
  assert.match(iosUnsignedBuilder, /--no-sign/);
  assert.match(iosUnsignedBuilder, /--archive-only/);
  assert.doesNotMatch(iosUnsignedBuilder, /^\s*xcodebuild(?:\s|\\)/m);
  assert.doesNotMatch(iosUnsignedBuilder, /is_signing_failure/);
  assert.match(collector, /'\.apk'/);
  assert.match(collector, /'\.aab'/);
  assert.match(collector, /'\.ipa'/);
  assert.match(collector, /android-arm64-apk/);
  assert.match(collector, /ios-arm64-unsigned-ipa/);

  const signingSecretPattern = /secrets\.(?:APPLE|IOS|APP_STORE|ANDROID|GOOGLE_PLAY|KEYSTORE)/i;
  assert.doesNotMatch(androidWorkflow, signingSecretPattern);
  assert.doesNotMatch(iosWorkflow, signingSecretPattern);
  assert.doesNotMatch(releaseWorkflow, signingSecretPattern);
});

test('bootstrap é interno, política de senha é central e mobile suporta modo local ou API', () => {
  const bootstrapSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'bootstrap.rs'),
    'utf8',
  );
  const securitySource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'security.rs'),
    'utf8',
  );
  const authSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'commands', 'auth.rs'),
    'utf8',
  );
  const apiSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'internal_api', 'app_commands.rs'),
    'utf8',
  );
  const apiDocsSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'internal_api', 'docs.rs'),
    'utf8',
  );
  const migrationSource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'migrations.rs'),
    'utf8',
  );
  const librarySource = fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'src', 'lib.rs'),
    'utf8',
  );
  const mobileConfig = JSON.parse(fs.readFileSync(
    path.join(repositoryRoot, 'src-tauri', 'mobile', 'mobile.config.json'),
    'utf8',
  ));
  const routerSource = fs.readFileSync(
    path.join(repositoryRoot, 'src', 'router', 'index.ts'),
    'utf8',
  );
  const mobileProvider = fs.readFileSync(
    path.join(repositoryRoot, 'src', 'core', 'invoker', 'MobileRemoteCommandProvider.ts'),
    'utf8',
  );
  const mobileConnection = fs.readFileSync(
    path.join(repositoryRoot, 'src', 'core', 'mobile', 'MobileConnection.ts'),
    'utf8',
  );
  const harness = fs.readFileSync(
    path.join(repositoryRoot, 'tests', 'legacy-migration-harness', 'src', 'main.rs'),
    'utf8',
  );
  const harnessCargo = fs.readFileSync(
    path.join(repositoryRoot, 'tests', 'legacy-migration-harness', 'Cargo.toml'),
    'utf8',
  );

  assert.match(bootstrapSource, /resolve_internal/);
  assert.match(bootstrapSource, /MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD/);
  assert.match(bootstrapSource, /remove_legacy_file/);
  assert.doesNotMatch(bootstrapSource, /OpenOptions|create_new\(true\)|write_all\(|File::create/);
  assert.doesNotMatch(bootstrapSource, /MANIFESTADOR_NFE_BOOTSTRAP_FILE/);
  assert.doesNotMatch(bootstrapSource, /current_dir\(\)/);
  assert.match(securitySource, /pub struct PasswordPolicy/);
  assert.match(securitySource, /PASSWORD_MIN_ALLOWED: usize = 6/);
  assert.match(authSource, /auth_initialize_master_password/);
  assert.match(authSource, /#\[cfg\(feature = "mobile"\)\][\s\S]*pub fn auth_local_guest/);
  assert.match(authSource, /authorize_api_command/);
  assert.match(authSource, /não está autorizado para execução pela API remota/);
  assert.match(authSource, /"api:access"/);
  assert.match(authSource, /"mobile:access"/);
  assert.match(apiSource, /company_allowed/);
  assert.match(apiSource, /auth_login_endpoint/);
  assert.match(apiSource, /Canal remoto inválido\. Use api ou mobile/);
  assert.match(apiSource, /auth_password_policy_endpoint/);
  assert.match(apiSource, /payload\.remove\("senha_hash"\)/);
  assert.match(apiSource, /user_record_for_api/);
  assert.match(apiSource, /O usuário master da sessão não pode ser inativado nem perder o perfil master/);
  assert.match(apiDocsSource, /"\/api\/auth\/login"/);
  assert.match(apiDocsSource, /"\/api\/auth\/password-policy"/);
  assert.match(apiDocsSource, /"\/api\/command"/);
  assert.match(migrationSource, /LOWER\(login\) = 'admin' OR administrador = 1/);
  assert.match(migrationSource, /\("usuarios", "administrador", "INTEGER NOT NULL DEFAULT 0"\)/);
  assert.match(migrationSource, /\("usuarios", "created_at", "TEXT NOT NULL DEFAULT ''"\)/);
  assert.match(migrationSource, /master_is_provisional/);
  assert.match(migrationSource, /environment_password_configured/);
  assert.match(migrationSource, /ensure_mobile_guest_seed/);
  assert.match(migrationSource, /rand::random::<u128>/);
  assert.doesNotMatch(migrationSource, /uuid::Uuid/);
  assert.doesNotMatch(harnessCargo, /^uuid\s*=/m);
  const desktopRun = librarySource.split('#[cfg(all(feature = "mobile", not(feature = "desktop")))]')[0];
  assert.doesNotMatch(desktopRun, /commands::auth::auth_local_guest/);
  assert.equal(mobileConfig.authentication.desktopBootstrap, 'internal');
  assert.equal(mobileConfig.authentication.localLoginRequiredByDefault, false);
  assert.equal(mobileConfig.authentication.remoteLoginRequired, true);
  assert.equal(mobileConfig.operationModes.local.authentication, 'optional');
  assert.deepEqual(
    [...mobileConfig.operationModes.remote.authorization].sort(),
    ['companies', 'permissions', 'profile'],
  );
  assert.match(routerSource, /\/mobile-inicio/);
  assert.match(routerSource, /canInitializePasswordLocally/);
  assert.match(routerSource, /mobileLocalAuthenticationRequired\(\)/);
  assert.match(mobileProvider, /\/api\/auth\/login/);
  assert.match(mobileProvider, /\/api\/auth\/password-policy/);
  assert.match(mobileProvider, /\/api\/command/);
  assert.match(mobileConnection, /sessionStorage/);
  assert.doesNotMatch(harness, /ferias_colaboradores|funcionarios|batidas|Ponto Manager/i);
});
