#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const platform = process.argv[2]?.trim().toLowerCase();
if (!['android', 'ios'].includes(platform)) {
  console.error('Uso: node scripts/mobile/generate-mobile-icons.mjs <android|ios>');
  process.exit(2);
}

const root = process.cwd();
const tauriDir = path.join(root, 'src-tauri');
const iconsDir = path.join(tauriDir, 'icons');
const mobileConfigPath = path.join(tauriDir, 'mobile', 'mobile.config.json');
const markerPath = path.join(tauriDir, 'gen', '.mobile-branding.json');
const preservedNames = new Set(['android', 'ios']);
let temporaryDirectory;

function sha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function readPngDimensions(filePath) {
  const buffer = fs.readFileSync(filePath);
  const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (buffer.length < 24 || !buffer.subarray(0, 8).equals(pngSignature)) {
    throw new Error(`O asset mobile deve ser PNG válido: ${path.relative(root, filePath)}.`);
  }
  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
}

function nativeIconFiles() {
  const base = platform === 'android'
    ? path.join(tauriDir, 'gen', 'android')
    : path.join(tauriDir, 'gen', 'apple');
  if (!fs.existsSync(base)) return [];

  const matches = [];
  const pending = [base];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(absolute);
      } else if (
        platform === 'android'
          ? /(?:ic_launcher|mipmap|launcher).+\.(?:png|webp|xml)$/i.test(absolute)
          : absolute.includes('AppIcon.appiconset') && /\.(?:png|json)$/i.test(absolute)
      ) {
        matches.push(absolute);
      }
    }
  }
  return matches;
}

function generatedIconFiles() {
  const directory = path.join(iconsDir, platform);
  if (!fs.existsSync(directory)) return [];
  const matches = [];
  const pending = [directory];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) pending.push(absolute);
      else matches.push(absolute);
    }
  }
  return matches;
}

function loadMarker() {
  try {
    return JSON.parse(fs.readFileSync(markerPath, 'utf8'));
  } catch {
    return {};
  }
}

function saveMarker(sourceHash) {
  fs.mkdirSync(path.dirname(markerPath), { recursive: true });
  const marker = loadMarker();
  marker.schemaVersion = 1;
  marker[platform] = {
    sourceSha256: sourceHash,
    generatedAt: new Date().toISOString(),
  };
  fs.writeFileSync(markerPath, `${JSON.stringify(marker, null, 2)}\n`, 'utf8');
}

function preserveDesktopIcons() {
  temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'tauri-mobile-icons-'));
  const backup = path.join(temporaryDirectory, 'desktop-icons');
  fs.mkdirSync(backup, { recursive: true });
  if (!fs.existsSync(iconsDir)) return backup;

  for (const entry of fs.readdirSync(iconsDir, { withFileTypes: true })) {
    if (preservedNames.has(entry.name)) continue;
    fs.cpSync(path.join(iconsDir, entry.name), path.join(backup, entry.name), {
      recursive: true,
      preserveTimestamps: true,
    });
  }
  return backup;
}

function restoreDesktopIcons(backup) {
  fs.mkdirSync(iconsDir, { recursive: true });
  for (const entry of fs.readdirSync(iconsDir, { withFileTypes: true })) {
    if (preservedNames.has(entry.name)) continue;
    fs.rmSync(path.join(iconsDir, entry.name), { recursive: true, force: true });
  }
  for (const entry of fs.readdirSync(backup, { withFileTypes: true })) {
    fs.cpSync(path.join(backup, entry.name), path.join(iconsDir, entry.name), {
      recursive: true,
      preserveTimestamps: true,
    });
  }
}

try {
  if (!fs.existsSync(mobileConfigPath)) {
    throw new Error(`Configuração mobile ausente: ${path.relative(root, mobileConfigPath)}.`);
  }
  const mobile = JSON.parse(fs.readFileSync(mobileConfigPath, 'utf8'));
  const androidSource = path.resolve(root, mobile.assets?.androidIconSource ?? '');
  const iosSource = path.resolve(root, mobile.assets?.iosIconSource ?? '');
  for (const source of [androidSource, iosSource]) {
    if (!fs.existsSync(source)) {
      throw new Error(`Fonte de ícone mobile ausente: ${path.relative(root, source)}.`);
    }
    const dimensions = readPngDimensions(source);
    if (dimensions.width !== 1024 || dimensions.height !== 1024) {
      throw new Error(
        `A fonte de ícone mobile deve medir 1024x1024: ${path.relative(root, source)} `
        + `mede ${dimensions.width}x${dimensions.height}.`,
      );
    }
  }

  const androidHash = sha256(androidSource);
  const iosHash = sha256(iosSource);
  if (androidHash !== iosHash) {
    throw new Error(
      'As fontes Android e iOS devem representar a mesma identidade visual. '
      + 'Gere novamente os assets mobile pelo scaffolder antes de inicializar a plataforma.',
    );
  }
  const sourceHash = platform === 'android' ? androidHash : iosHash;
  const sourcePath = platform === 'android' ? androidSource : iosSource;
  const marker = loadMarker();
  if (
    marker[platform]?.sourceSha256 === sourceHash
    && (generatedIconFiles().length > 0 || nativeIconFiles().length > 0)
  ) {
    console.log(`Ícones ${platform} já estão sincronizados com o branding atual.`);
    process.exit(0);
  }

  const backup = preserveDesktopIcons();
  const wrapper = path.join(root, 'scripts', 'ci', 'run-tauri.mjs');
  const result = spawnSync(process.execPath, [wrapper, 'icon', sourcePath], {
    cwd: root,
    env: process.env,
    stdio: 'inherit',
  });
  try {
    restoreDesktopIcons(backup);
  } catch (error) {
    throw new Error(`Falha ao restaurar os ícones desktop preservados: ${error.message}`);
  }
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`tauri icon terminou com código ${result.status}.`);
  }

  const generated = generatedIconFiles();
  const native = nativeIconFiles();
  if (generated.length === 0 && native.length === 0) {
    throw new Error(`O comando tauri icon não produziu variantes reconhecíveis para ${platform}.`);
  }
  saveMarker(sourceHash);
  console.log(
    `Branding ${platform} sincronizado (${generated.length} assets Tauri; `
    + `${native.length} assets no projeto nativo).`,
  );
} catch (error) {
  console.error(`Falha ao gerar ícones mobile: ${error.message}`);
  process.exitCode = 1;
} finally {
  if (temporaryDirectory) {
    fs.rmSync(temporaryDirectory, { recursive: true, force: true });
  }
}
