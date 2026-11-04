import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const version = process.argv[2];
if (!version) {
  console.error('Uso: node scripts/release/prepare-release.mjs <version>');
  process.exit(1);
}

const root = process.cwd();

function writeJson(filePath, data) {
  fs.writeFileSync(filePath, `${JSON.stringify(data, null, 2)}
`);
}

function updatePackageJson() {
  const filePath = path.join(root, 'package.json');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  data.version = version;
  writeJson(filePath, data);
}

function updatePackageLock() {
  const filePath = path.join(root, 'package-lock.json');
  if (!fs.existsSync(filePath)) {
    return;
  }
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  data.version = version;
  if (data.packages && data.packages['']) {
    data.packages[''].version = version;
  }
  writeJson(filePath, data);
}

function updateTauriConf() {
  const filePath = path.join(root, 'src-tauri', 'tauri.conf.json');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  data.version = version;
  writeJson(filePath, data);
}

function updateProjectConfig() {
  const filePath = path.join(root, 'src', 'config', 'projectConfig.ts');
  if (!fs.existsSync(filePath)) {
    return;
  }
  const content = fs.readFileSync(filePath, 'utf8');
  const updated = content.replace(/version:\s*["'][^"']+["']/, `version: "${version}"`);
  fs.writeFileSync(filePath, updated);
}

function updateCargoToml() {
  const filePath = path.join(root, 'src-tauri', 'Cargo.toml');
  const content = fs.readFileSync(filePath, 'utf8');
  const updated = content.replace(/(\[package\][\s\S]*?^version\s*=\s*")([^"]+)(")/m, `$1${version}$3`);
  fs.writeFileSync(filePath, updated);
}

function cargoPackageName() {
  const filePath = path.join(root, 'src-tauri', 'Cargo.toml');
  const content = fs.readFileSync(filePath, 'utf8');
  const packageSection = content.match(/\[package\]([\s\S]*?)(?:\n\[|$)/);
  const packageName = packageSection?.[1].match(/^name\s*=\s*"([^"]+)"/m)?.[1];
  if (!packageName) {
    throw new Error('Nome do pacote Rust não encontrado em src-tauri/Cargo.toml.');
  }
  return packageName;
}

function updateCargoLock() {
  const filePath = path.join(root, 'src-tauri', 'Cargo.lock');
  if (!fs.existsSync(filePath)) {
    throw new Error('src-tauri/Cargo.lock é obrigatório antes de criar a tag.');
  }

  const packageName = cargoPackageName();
  const content = fs.readFileSync(filePath, 'utf8');
  const blocks = content.split(/(?=^\[\[package\]\]$)/m);
  let updates = 0;
  const updated = blocks.map((block) => {
    const name = block.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
    if (name !== packageName || /^source\s*=/m.test(block)) {
      return block;
    }
    updates += 1;
    return block.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  }).join('');

  if (updates !== 1) {
    throw new Error(
      `Esperado um pacote workspace ${packageName} no Cargo.lock; encontrados: ${updates}.`,
    );
  }
  fs.writeFileSync(filePath, updated);

  const cargo = process.env.CARGO?.trim() || 'cargo';
  const verification = spawnSync(cargo, [
    'metadata',
    '--manifest-path',
    path.join('src-tauri', 'Cargo.toml'),
    '--locked',
    '--format-version',
    '1',
  ], {
    cwd: root,
    encoding: 'utf8',
    stdio: 'inherit',
  });
  if (verification.error) {
    throw new Error(`Não foi possível validar Cargo.lock: ${verification.error.message}`);
  }
  if (verification.status !== 0) {
    throw new Error(`Cargo.lock permaneceu inconsistente (cargo metadata: ${verification.status}).`);
  }
}

fs.writeFileSync(path.join(root, 'VERSION'), `${version}
`);
updatePackageJson();
updatePackageLock();
updateTauriConf();
updateCargoToml();
updateCargoLock();
updateProjectConfig();
console.log(`Versão atualizada para ${version}`);
