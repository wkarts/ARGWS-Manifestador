// Numeric product versions; the GitHub prerelease flag carries the channel.
export function parseVersion(value) {
  const match = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(value);
  if (!match) throw new Error(`Versão numérica SemVer inválida: ${value}`);
  return match.slice(1).map(BigInt);
}
export function compare(a, b) {
  const left = parseVersion(a), right = parseVersion(b);
  for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] > right[i] ? 1 : -1;
  return 0;
}
function increment(version, type = 'patch') {
  const parts = parseVersion(version);
  const index = { major: 0, minor: 1, patch: 2 }[type];
  if (index === undefined) throw new Error(`Incremento inválido: ${type}`);
  parts[index]++;
  for (let i = index + 1; i < 3; i++) parts[i] = 0n;
  return parts.join('.');
}
export function planVersion({ branch, current, tags, source, bump = 'patch' }) {
  if (!['main', 'develop'].includes(branch)) throw new Error('Canal permitido: main ou develop.');
  parseVersion(current);
  if (!/^[a-f0-9]{40}$/.test(source)) throw new Error('SHA de origem inválido.');
  const numeric = tags.filter(t => /^v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/.test(t.tag));
  const existing = numeric.find(t => t.source === source && t.channel === branch);
  if (existing) return { version: existing.tag.slice(1), tag: existing.tag, prerelease: branch === 'develop', existing: true };
  // Unknown historical tags are conservatively reserved as stable versions.
  const stable = numeric.filter(t => t.channel !== 'develop').sort((a, b) => compare(a.tag, b.tag)).at(-1);
  let version = stable ? increment(stable.tag, bump) : increment(current, bump);
  if (compare(current, version) > 0) version = current;
  const highest = numeric.sort((a, b) => compare(a.tag, b.tag)).at(-1);
  if (highest && compare(version, highest.tag) <= 0) version = increment(highest.tag);
  return { version, tag: `v${version}`, prerelease: branch === 'develop', existing: false };
}
