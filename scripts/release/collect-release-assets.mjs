#!/usr/bin/env node

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const ALLOWED_SUFFIXES = [
  '.AppImage',
  '.aab',
  '.apk',
  '.deb',
  '.dmg',
  '.exe',
  '.msi',
  '.ipa',
  '.rpm',
  '.sig',
  '.tar.gz',
  '.zip',
];

const EXPECTED_TARGETS = [
  { id: 'desktop-windows-x64', label: 'Windows x64', platform: 'windows', arch: 'x64', target: 'x86_64-pc-windows-msvc', artifact: 'release-desktop-windows-x64' },
  { id: 'desktop-windows-x86', label: 'Windows x86', platform: 'windows', arch: 'x86', target: 'i686-pc-windows-msvc', artifact: 'release-desktop-windows-x86' },
  { id: 'desktop-linux-x64', label: 'Linux x64', platform: 'linux', arch: 'x64', target: 'x86_64-unknown-linux-gnu', artifact: 'release-desktop-linux-x64' },
  { id: 'desktop-linux-arm64', label: 'Linux ARM64', platform: 'linux', arch: 'arm64', target: 'aarch64-unknown-linux-gnu', artifact: 'release-desktop-linux-arm64' },
  { id: 'desktop-macos-arm64', label: 'macOS Apple Silicon', platform: 'macos', arch: 'arm64', target: 'aarch64-apple-darwin', artifact: 'release-desktop-macos-arm64' },
  { id: 'desktop-macos-x64', label: 'macOS Intel', platform: 'macos', arch: 'x64', target: 'x86_64-apple-darwin', artifact: 'release-desktop-macos-x64' },
  { id: 'web-pwa', label: 'Web/PWA', platform: 'web', arch: 'universal', target: 'web', artifact: 'release-web-pwa' },
  { id: 'cloudpanel-linux-x64', label: 'CloudPanel Linux x64', platform: 'cloudpanel-linux', arch: 'x64', target: 'x86_64-unknown-linux-gnu', artifact: 'release-cloudpanel-linux-x64' },
  { id: 'cloudpanel-linux-x86', label: 'CloudPanel Linux x86', platform: 'cloudpanel-linux', arch: 'x86', target: 'i686-unknown-linux-gnu', artifact: 'release-cloudpanel-linux-x86' },
  { id: 'android-arm64-apk', label: 'Android APK ARM64', platform: 'android', arch: 'arm64', target: 'aarch64-linux-android', artifact: 'release-android-arm64-apk' },
  { id: 'android-aab', label: 'Android AAB', platform: 'android', arch: 'arm64', target: 'android-aab', artifact: 'release-android-aab' },
  { id: 'ios-arm64-unsigned-ipa', label: 'iOS ARM64 IPA não assinado', platform: 'ios', arch: 'arm64', target: 'aarch64-apple-ios', artifact: 'release-ios-arm64-unsigned-ipa' },
];

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
    if (entry.isDirectory()) {
      files.push(...walk(absolute));
    } else if (entry.isFile()) {
      files.push(absolute);
    }
  }
  return files;
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

function isReleaseAsset(file) {
  if (file.endsWith('.sha256')) {
    return false;
  }
  return ALLOWED_SUFFIXES.some((suffix) => file.endsWith(suffix));
}

function readProjectIdentity() {
  const packageJson = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  const version = fs.readFileSync('VERSION', 'utf8').trim();
  return {
    product: sanitize(packageJson.name),
    version,
  };
}

function escapeMarkdown(value) {
  return String(value ?? '').replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function readBuildMatrix(input, assetContexts) {
  const statusFiles = walk(input).filter((file) => file.endsWith('.status.json')).sort();
  const recorded = new Map();

  for (const file of statusFiles) {
    let status;
    try {
      status = JSON.parse(fs.readFileSync(file, 'utf8'));
    } catch (error) {
      throw new Error(`Status de build inválido em ${file}: ${error.message}`);
    }
    if (!status.id || recorded.has(status.id)) {
      throw new Error(`Status de build ausente ou duplicado: ${status.id ?? file}`);
    }
    recorded.set(status.id, status);
  }

  const targets = EXPECTED_TARGETS.map((expected) => {
    const status = recorded.get(expected.id);
    const artifactPresent = assetContexts.has(expected.artifact);
    if (!status) {
      return {
        ...expected,
        status: 'not-reported',
        success: false,
        artifactPresent,
        runUrl: null,
        error: `O job ${expected.label} não enviou o relatório de status. Consulte a execução do workflow.`,
      };
    }

    const success = status.status === 'success' && artifactPresent;
    return {
      ...expected,
      status: success ? 'success' : status.status,
      success,
      artifactPresent,
      runUrl: status.runUrl ?? null,
      error: success
        ? null
        : status.status === 'success'
          ? `O job ${expected.label} informou sucesso, mas o artefato ${expected.artifact} não foi encontrado.`
          : status.error ?? `O job ${expected.label} terminou com status ${status.status ?? 'unknown'}.`,
    };
  });

  const succeeded = targets.filter((target) => target.success).length;
  return {
    outcome: succeeded === targets.length ? 'complete' : 'partial',
    total: targets.length,
    succeeded,
    failed: targets.length - succeeded,
    targets,
  };
}

function buildStatusMarkdown(matrix) {
  const lines = [
    '## Estado da matriz de build',
    '',
    matrix.outcome === 'complete'
      ? 'Todos os alvos obrigatórios foram gerados.'
      : 'Matriz parcial: os artefatos aprovados estão disponíveis no draft; os alvos abaixo marcados como falha não foram incluídos.',
    '',
    '| Alvo | Estado | Artefato | Diagnóstico |',
    '| --- | --- | --- | --- |',
  ];
  for (const target of matrix.targets) {
    lines.push(
      `| ${escapeMarkdown(target.label)} | ${target.success ? 'gerado' : escapeMarkdown(target.status)} | ${target.artifactPresent ? escapeMarkdown(target.artifact) : 'não gerado'} | ${target.success ? '-' : escapeMarkdown(target.error)} |`,
    );
  }
  const firstRunUrl = matrix.targets.find((target) => target.runUrl)?.runUrl;
  if (firstRunUrl) {
    lines.push('', `Logs da execução: ${firstRunUrl}`);
  }
  return `${lines.join('\n')}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const identity = readProjectIdentity();
  const input = path.resolve(args.input ?? 'artifacts');
  const output = path.resolve(args.output ?? 'release-assets');
  const product = sanitize(args.product ?? identity.product);
  const version = sanitize(args.version ?? identity.version);

  if (!fs.existsSync(input) || !fs.statSync(input).isDirectory()) {
    throw new Error(`Diretório de entrada ausente: ${input}`);
  }
  if (!product || !version) {
    throw new Error('Produto e versão são obrigatórios para nomear os artefatos.');
  }

  fs.mkdirSync(output, { recursive: true });
  if (fs.readdirSync(output).length > 0) {
    throw new Error(`Diretório de saída deve estar vazio: ${output}`);
  }

  const selected = walk(input).filter(isReleaseAsset).sort();
  if (selected.length === 0) {
    throw new Error(`Nenhum artefato distribuível encontrado em ${input}`);
  }

  const names = new Set();
  const assets = [];

  for (const source of selected) {
    const relative = path.relative(input, source);
    if (relative.startsWith('..') || path.isAbsolute(relative)) {
      throw new Error(`Artefato fora do diretório de entrada: ${source}`);
    }

    const [artifactDirectory = 'artifact'] = relative.split(path.sep);
    const context = sanitize(artifactDirectory.replace(/^release-/, ''));
    const originalName = sanitize(path.basename(source));
    const outputName = `${product}-v${version}-${context}-${originalName}`;

    if (names.has(outputName)) {
      throw new Error(`Colisão de nome de artefato: ${outputName}`);
    }
    names.add(outputName);

    const destination = path.join(output, outputName);
    fs.copyFileSync(source, destination, fs.constants.COPYFILE_EXCL);

    const stat = fs.statSync(destination);
    assets.push({
      name: outputName,
      source: relative.split(path.sep).join('/'),
      size: stat.size,
      sha256: await sha256(destination),
    });
  }

  const checksumLines = assets.map((asset) => `${asset.sha256}  ${asset.name}`);
  const assetContexts = new Set(assets.map((asset) => asset.source.split('/')[0]));
  const buildMatrix = readBuildMatrix(input, assetContexts);
  const generatedAt = new Date().toISOString();
  fs.writeFileSync(
    path.join(output, 'SHA256SUMS.txt'),
    `${checksumLines.join('\n')}\n`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(output, 'RELEASE-MANIFEST.json'),
    `${JSON.stringify({
      product,
      version: args.version ?? identity.version,
      generatedAt,
      buildMatrix,
      assets,
    }, null, 2)}\n`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(output, 'RELEASE-STATUS.json'),
    `${JSON.stringify({ generatedAt, ...buildMatrix }, null, 2)}\n`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(output, 'RELEASE-STATUS.md'),
    buildStatusMarkdown(buildMatrix),
    'utf8',
  );

  console.log(`Artefatos coletados: ${assets.length}`);
  for (const asset of assets) {
    console.log(`${asset.sha256}  ${asset.name}  ${asset.size} bytes`);
  }
  if (buildMatrix.outcome === 'partial') {
    console.warn(`Release parcial: ${buildMatrix.succeeded}/${buildMatrix.total} alvos gerados.`);
  }
}

try {
  await main();
} catch (error) {
  console.error(`Falha ao coletar artefatos: ${error.message}`);
  process.exit(1);
}
