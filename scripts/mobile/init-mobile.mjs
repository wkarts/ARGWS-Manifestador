#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const platform = process.argv[2]?.trim().toLowerCase();
if (!['android', 'ios'].includes(platform)) {
  console.error('Uso: node scripts/mobile/init-mobile.mjs <android|ios>');
  process.exit(2);
}

if (platform === 'ios' && process.platform !== 'darwin') {
  console.error('A inicialização iOS exige macOS com Xcode instalado.');
  process.exit(2);
}

const root = process.cwd();
const tauriDir = path.join(root, 'src-tauri');
const generatedDir = platform === 'android'
  ? path.join(tauriDir, 'gen', 'android')
  : path.join(tauriDir, 'gen', 'apple');

function initialized() {
  if (!fs.existsSync(generatedDir)) return false;
  if (platform === 'android') {
    return fs.existsSync(path.join(generatedDir, 'gradlew'))
      && fs.existsSync(path.join(generatedDir, 'app'));
  }
  return fs.readdirSync(generatedDir, { withFileTypes: true })
    .some((entry) => entry.isDirectory() && (entry.name.endsWith('.xcodeproj') || entry.name.endsWith('.xcworkspace')));
}

function assertConfiguration() {
  const baseConfigPath = path.join(tauriDir, 'tauri.conf.json');
  const mobileConfigPath = path.join(tauriDir, 'mobile', 'mobile.config.json');
  if (!fs.existsSync(baseConfigPath) || !fs.existsSync(mobileConfigPath)) {
    throw new Error('Configuração mobile incompleta: tauri.conf.json ou mobile.config.json ausente.');
  }
  const base = JSON.parse(fs.readFileSync(baseConfigPath, 'utf8'));
  const mobile = JSON.parse(fs.readFileSync(mobileConfigPath, 'utf8'));
  const expectedIdentifier = platform === 'android'
    ? mobile.android?.package
    : mobile.ios?.bundleIdentifier;
  if (!base.identifier || base.identifier !== mobile.identifier) {
    throw new Error(`Identifier base divergente: tauri=${base.identifier ?? '(ausente)'}, mobile=${mobile.identifier ?? '(ausente)'}.`);
  }
  const platformConfig = JSON.parse(fs.readFileSync(
    path.join(tauriDir, `tauri.${platform === 'android' ? 'android' : 'ios'}.conf.json`),
    'utf8',
  ));
  if (platformConfig.identifier !== expectedIdentifier) {
    throw new Error(`Identifier divergente para ${platform}: configuração=${platformConfig.identifier ?? '(ausente)'}, mobile=${expectedIdentifier ?? '(ausente)'}.`);
  }
  if (JSON.stringify(platformConfig.build?.features ?? []) !== JSON.stringify(['mobile'])) {
    throw new Error(`A configuração ${platform} deve ativar somente a feature Cargo mobile.`);
  }
  const iconKey = platform === 'android' ? 'androidIconSource' : 'iosIconSource';
  const iconSource = mobile.assets?.[iconKey];
  if (!iconSource || !fs.existsSync(path.join(root, iconSource))) {
    throw new Error(`Asset mobile ausente para ${platform}: ${iconSource ?? iconKey}.`);
  }
}

try {
  assertConfiguration();
  const wasInitialized = initialized();
  if (!wasInitialized) {
    const wrapper = path.join(root, 'scripts', 'ci', 'run-tauri.mjs');
    const result = spawnSync(
      process.execPath,
      [wrapper, platform, 'init', '--ci', '--skip-targets-install'],
      {
        cwd: root,
        env: process.env,
        stdio: 'inherit',
      },
    );
    if (result.error) throw result.error;
    if (result.status !== 0) {
      throw new Error(`tauri ${platform} init terminou com código ${result.status}.`);
    }
    if (!initialized()) {
      throw new Error(`Tauri informou sucesso, mas o projeto gerado não foi encontrado em ${generatedDir}.`);
    }
    assertConfiguration();
  }

  const iconScript = path.join(root, 'scripts', 'mobile', 'generate-mobile-icons.mjs');
  const iconResult = spawnSync(process.execPath, [iconScript, platform], {
    cwd: root,
    env: process.env,
    stdio: 'inherit',
  });
  if (iconResult.error) throw iconResult.error;
  if (iconResult.status !== 0) {
    throw new Error(`A geração dos ícones ${platform} terminou com código ${iconResult.status}.`);
  }

  console.log(
    wasInitialized
      ? `Projeto Tauri ${platform} já inicializado e branding sincronizado em ${path.relative(root, generatedDir)}.`
      : `Projeto Tauri ${platform} inicializado em ${path.relative(root, generatedDir)}.`,
  );
} catch (error) {
  console.error(`Falha ao inicializar ${platform}: ${error.message}`);
  process.exit(1);
}
