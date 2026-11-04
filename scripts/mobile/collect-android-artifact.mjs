#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith('--') || !value || value.startsWith('--')) {
      throw new Error(`Argumento inválido ou sem valor: ${key ?? '(ausente)'}`);
    }
    result[key.slice(2)] = value;
  }
  return result;
}

function required(options, key) {
  const value = options[key]?.trim();
  if (!value) throw new Error(`--${key} é obrigatório.`);
  return value;
}

function sanitize(value) {
  return value
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^A-Za-z0-9._+-]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^-+|-+$/g, '');
}

function walk(directory) {
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...walk(absolute));
    else if (entry.isFile()) files.push(absolute);
  }
  return files;
}

function score(file, kind) {
  const normalized = file.split(path.sep).join('/').toLowerCase();
  let value = 0;
  if (normalized.includes('/outputs/')) value += 20;
  if (normalized.includes('/release/')) value += 15;
  if (normalized.includes('/debug/')) value += kind === 'apk' ? 18 : -30;
  if (normalized.includes('universal')) value += 8;
  if (normalized.includes('arm64') || normalized.includes('aarch64')) value += 6;
  if (normalized.includes('/intermediates/') || normalized.includes('androidtest')) value -= 100;
  if (normalized.includes('unaligned')) value -= 50;
  return value;
}

function archiveEntries(file) {
  const result = spawnSync('unzip', ['-Z1', file], { encoding: 'utf8' });
  if (result.error) throw new Error(`Não foi possível executar unzip: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`Arquivo Android inválido: ${file}\n${result.stderr}`);
  return result.stdout.split(/\r?\n/).filter(Boolean);
}

function sha256(file) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(file));
  return hash.digest('hex');
}

function isJarSignatureEntry(entry) {
  const normalized = entry.replaceAll('\\', '/').toUpperCase();
  return /^META-INF\/[^/]+\.(SF|RSA|DSA|EC|SIG)$/.test(normalized);
}

try {
  const options = parseArgs(process.argv.slice(2));
  const kind = required(options, 'kind').toLowerCase();
  if (!['apk', 'aab'].includes(kind)) throw new Error('--kind deve ser apk ou aab.');
  const input = path.resolve(required(options, 'input'));
  const output = path.resolve(required(options, 'output'));
  const product = sanitize(required(options, 'product'));
  const version = sanitize(required(options, 'version'));
  if (!fs.existsSync(input) || !fs.statSync(input).isDirectory()) {
    throw new Error(`Diretório de outputs Android ausente: ${input}`);
  }

  const candidates = walk(input)
    .filter((file) => file.toLowerCase().endsWith(`.${kind}`))
    .sort((a, b) => score(b, kind) - score(a, kind) || a.localeCompare(b));
  const selected = candidates[0];
  if (!selected || score(selected, kind) < 0) {
    throw new Error(`Nenhum ${kind.toUpperCase()} distribuível foi encontrado em ${input}.`);
  }

  const entries = archiveEntries(selected);
  if (kind === 'apk') {
    if (!entries.includes('AndroidManifest.xml')) throw new Error('APK sem AndroidManifest.xml.');
    if (!entries.some((entry) => entry.startsWith('lib/arm64-v8a/'))) {
      throw new Error('APK não contém biblioteca nativa ARM64 (lib/arm64-v8a).');
    }
  } else {
    if (!entries.includes('base/manifest/AndroidManifest.xml')) throw new Error('AAB sem manifesto base.');
    if (!entries.some((entry) => entry.startsWith('base/lib/arm64-v8a/'))) {
      throw new Error('AAB não contém biblioteca nativa ARM64 (base/lib/arm64-v8a).');
    }
    const signatureEntry = entries.find(isJarSignatureEntry);
    if (signatureEntry) {
      throw new Error(`AAB contém assinatura JAR inesperada (${signatureEntry}); o artefato não pode ser publicado como unsigned.`);
    }
  }

  fs.mkdirSync(output, { recursive: true });
  const name = kind === 'apk'
    ? `${product}-android-arm64-v${version}-debug.apk`
    : `${product}-android-v${version}-unsigned.aab`;
  const destination = path.join(output, name);
  fs.copyFileSync(selected, destination);
  const digest = sha256(destination);
  fs.writeFileSync(`${destination}.sha256`, `${digest}  ${name}\n`, 'utf8');
  console.log(destination);
} catch (error) {
  console.error(`Falha ao coletar artefato Android: ${error.message}`);
  process.exit(1);
}
