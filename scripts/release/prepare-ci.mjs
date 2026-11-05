import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import { analyzeCommits } from '@semantic-release/commit-analyzer';
import { generateNotes } from '@semantic-release/release-notes-generator';
import { planVersion, compare } from './version-plan.mjs';

const run = (command, args) => execFileSync(command, args, { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 }).trim();
const git = (...args) => run('git', args);
const branch = process.env.RELEASE_BRANCH;
if (!['main', 'develop'].includes(branch)) throw new Error('Release exige origem main ou develop.');
const source = git('rev-parse', 'HEAD');
if (process.env.RELEASE_SOURCE_SHA && source !== process.env.RELEASE_SOURCE_SHA) throw new Error('Checkout difere do SHA aprovado no CI.');
if (!process.env.GITHUB_OUTPUT) throw new Error('GITHUB_OUTPUT obrigatório.');
const output = (values) => fs.appendFileSync(process.env.GITHUB_OUTPUT, Object.entries(values).map(([key, value]) => `${key}=${value}\n`).join(''));
// Read-only fetch: never force an existing tag or move a branch.
git('fetch', 'origin', branch, '--tags');
const tip = git('rev-parse', 'FETCH_HEAD');
if (source !== tip) {
  output({ published: false });
  console.log('CI de commit antigo: nenhuma versão será criada.');
  process.exit(0);
}
const tags = git('tag', '--list', 'v*').split('\n').filter(Boolean).map(tag => {
  const contents = git('for-each-ref', '--format=%(contents)', `refs/tags/${tag}`);
  const marker = contents.match(/^Manifestador-Release: (.+)$/m);
  let metadata = {};
  if (marker) {
    metadata = JSON.parse(marker[1]);
    if (!['main', 'develop'].includes(metadata.channel) || !/^[a-f0-9]{40}$/.test(metadata.source)) throw new Error(`Metadados inválidos em ${tag}`);
    if (git('rev-parse', `${tag}^`) !== metadata.source) throw new Error(`Origem da tag ${tag} não confere.`);
  }
  return { tag, ...metadata };
});
const stable = tags.filter(t => /^v\d+\.\d+\.\d+$/.test(t.tag) && t.channel !== 'develop').sort((a, b) => compare(a.tag, b.tag)).at(-1);
const messages = git('log', '--format=%H%x1f%B%x00', stable ? `${stable.tag}..${source}` : source).split('\0').map(entry => {
  const split = entry.indexOf('\x1f');
  return { hash: entry.slice(0, split).trim(), message: entry.slice(split + 1).trim() };
}).filter(c => c.hash && c.message);
const bump = await analyzeCommits({}, { commits: messages, logger: console }) || 'patch';
const plan = planVersion({ branch, current: fs.readFileSync('VERSION', 'utf8').trim(), source, tags, bump });
// A rerun of a published version is a no-op, never a rebuild/overwrite.
const releases = JSON.parse(run('gh', ['api', '--paginate', '--slurp', `repos/${process.env.GITHUB_REPOSITORY}/releases?per_page=100`])).flat();
const release = releases.find(r => r.tag_name === plan.tag);
if (release && release.prerelease !== plan.prerelease) throw new Error('Canal da release não corresponde à tag.');
if (release && !release.draft) {
  output({ published: false, version: plan.version, tag: plan.tag, prerelease: plan.prerelease });
  console.log(`Release ${plan.tag} já publicada; nenhuma alteração.`);
  process.exit(0);
}
const generatedNotes = await generateNotes({}, {
  cwd: process.cwd(), commits: messages, logger: console,
  lastRelease: stable ? { gitTag: stable.tag, version: stable.tag.slice(1) } : {},
  nextRelease: { gitTag: plan.tag, version: plan.version, gitHead: source },
  options: { repositoryUrl: `${process.env.GITHUB_SERVER_URL || 'https://github.com'}/${process.env.GITHUB_REPOSITORY}` },
});
const notes = `${generatedNotes}\nCanal: ${branch}\nCommit de origem aprovado no CI: ${source}\n`;
if (!plan.existing) {
  execFileSync(process.execPath, ['scripts/release/prepare-release.mjs', plan.version], { stdio: 'inherit' });
  execFileSync(process.execPath, ['scripts/ci/check-version-sync.mjs', '--tag', plan.tag], { stdio: 'inherit' });
  git('config', 'user.name', 'github-actions[bot]');
  git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com');
  let changelog = fs.existsSync('CHANGELOG.md') ? fs.readFileSync('CHANGELOG.md', 'utf8') : '';
  if (stable) {
    try { changelog = git('show', `${stable.tag}:CHANGELOG.md`); } catch { /* Historical tag without changelog: preserve the checked-out file. */ }
  }
  fs.writeFileSync('CHANGELOG.md', `${notes}\n${changelog}\n`);
  git('add', 'CHANGELOG.md', 'VERSION', 'package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json', 'src/config/projectConfig.ts');
  git('commit', '--allow-empty', '-m', `chore(release): ${plan.version} [skip ci]`);
  git('tag', '-a', plan.tag, '-m', `Manifestador NF-e ${plan.version}\nManifestador-Release: ${JSON.stringify({ source, channel: branch })}`);
  // A concurrent reservation fails here rather than overwriting someone else's tag.
  git('push', 'origin', `refs/tags/${plan.tag}`);
}

output({ published: true, version: plan.version, tag: plan.tag, name: `Manifestador NF-e v${plan.version}`, prerelease: plan.prerelease, notes_b64: Buffer.from(notes).toString('base64') });
