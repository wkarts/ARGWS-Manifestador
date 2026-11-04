#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const current = argv[index];
    if (!current.startsWith('--')) {
      throw new Error(`Argumento inválido: ${current}`);
    }
    const key = current.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) {
      throw new Error(`Valor ausente para --${key}`);
    }
    options[key] = value;
    index += 1;
  }
  return options;
}

function runGh(args, options = {}) {
  const result = spawnSync('gh', args, {
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  });
  if (result.error) {
    throw new Error(`Não foi possível executar gh: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const details = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim();
    throw new Error(`gh ${args.join(' ')} falhou: ${details}`);
  }
  return result.stdout ?? '';
}

function sha256(file) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(file);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

function releaseState(tag) {
  return JSON.parse(runGh(['release', 'view', tag, '--json', 'isDraft,assets']));
}

async function compareExistingAsset(tag, name, localFile) {
  const tempDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'release-asset-'));
  try {
    runGh([
      'release',
      'download',
      tag,
      '--pattern',
      name,
      '--dir',
      tempDirectory,
    ]);
    const downloaded = path.join(tempDirectory, name);
    if (!fs.existsSync(downloaded)) {
      throw new Error(`O asset remoto ${name} não foi baixado para comparação.`);
    }
    return await sha256(downloaded) === await sha256(localFile);
  } finally {
    fs.rmSync(tempDirectory, { recursive: true, force: true });
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const tag = args.tag?.trim();
  const directory = path.resolve(args.directory ?? 'release-assets');

  if (!tag || !/^v[0-9]+\.[0-9]+\.[0-9]+(?:[+-][0-9A-Za-z.-]+)?$/.test(tag)) {
    throw new Error(`Tag inválida: ${tag ?? '(ausente)'}`);
  }
  if (!process.env.GH_TOKEN && !process.env.GITHUB_TOKEN) {
    throw new Error('GH_TOKEN ou GITHUB_TOKEN não está definido.');
  }
  if (!fs.existsSync(directory) || !fs.statSync(directory).isDirectory()) {
    throw new Error(`Diretório de assets ausente: ${directory}`);
  }

  const files = fs.readdirSync(directory)
    .map((name) => path.join(directory, name))
    .filter((file) => fs.statSync(file).isFile())
    .sort();
  if (files.length === 0) {
    throw new Error(`Nenhum arquivo para publicar em ${directory}`);
  }

  let state = releaseState(tag);
  if (!state.isDraft) {
    throw new Error(`A release ${tag} já está publicada e não será reaberta.`);
  }

  const existing = new Map((state.assets ?? []).map((asset) => [asset.name, asset]));
  for (const file of files) {
    const name = path.basename(file);
    if (existing.has(name)) {
      if (!await compareExistingAsset(tag, name, file)) {
        throw new Error(
          `O asset remoto ${name} existe com conteúdo diferente; publicação interrompida.`,
        );
      }
      console.log(`Asset já completo e idêntico: ${name}`);
      continue;
    }

    runGh(['release', 'upload', tag, file]);
    console.log(`Asset enviado: ${name}`);
  }

  state = releaseState(tag);
  const publishedNames = new Set((state.assets ?? []).map((asset) => asset.name));
  const missing = files
    .map((file) => path.basename(file))
    .filter((name) => !publishedNames.has(name));
  if (missing.length > 0) {
    throw new Error(`Assets ausentes após o upload: ${missing.join(', ')}`);
  }

  console.log(`Release ${tag}: ${files.length} assets validados no draft.`);
}

try {
  await main();
} catch (error) {
  console.error(`Falha ao publicar assets: ${error.message}`);
  process.exit(1);
}
