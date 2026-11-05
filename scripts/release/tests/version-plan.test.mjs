import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { planVersion, parseVersion } from '../version-plan.mjs';
const source = 'a'.repeat(40);
const base = { branch: 'develop', current: '0.2.0', source, tags: [{ tag: 'v1.0.0' }] };
test('develop produces numeric next version above stable or reserved draft', () => {
  assert.deepEqual(planVersion(base), { version: '1.0.1', tag: 'v1.0.1', prerelease: true, existing: false });
});
test('successive commits reserve new versions and stable never collides with preview', () => {
  const tags = [...base.tags, { tag: 'v1.0.1', channel: 'develop', source: 'b'.repeat(40) }];
  assert.equal(planVersion({ ...base, tags }).version, '1.0.2');
  assert.deepEqual(planVersion({ ...base, branch: 'main', tags }), { version: '1.0.2', tag: 'v1.0.2', prerelease: false, existing: false });
});
test('same source and channel reuse the immutable reservation, not a new version', () => {
  const tags = [...base.tags, { tag: 'v1.0.1', channel: 'develop', source }];
  assert.equal(planVersion({ ...base, tags }).existing, true);
  assert.equal(planVersion({ ...base, tags, branch: 'main' }).existing, false);
});
test('Conventional Commit minor and major increments are preserved', () => {
  assert.equal(planVersion({ ...base, bump: 'minor' }).version, '1.1.0');
  assert.equal(planVersion({ ...base, bump: 'major' }).version, '2.0.0');
  assert.equal(planVersion({ ...base, current: '3.0.0' }).version, '3.0.0');
});
test('first release and strict SemVer validation', () => {
  assert.equal(planVersion({ ...base, tags: [] }).version, '0.2.1');
  for (const version of ['1.0.1-beta.1', '01.0.0', '1.0', '1.0.0+build']) assert.throws(() => parseVersion(version));
  assert.throws(() => planVersion({ ...base, branch: 'feature/test' }));
  assert.throws(() => planVersion({ ...base, source: 'invalid' }));
});
test('workflow supports approved same-repository develop CI and preserves channel on resume/publish', () => {
  const workflow = fs.readFileSync('.github/workflows/release.yml', 'utf8');
  assert.match(workflow, /branches:\s+- main\s+- develop/);
  assert.match(workflow, /head_repository.full_name == github.repository/);
  assert.match(workflow, /workflow_run.event == 'push'/);
  assert.match(workflow, /node .\/scripts\/release\/prepare-ci.mjs/);
  assert.match(workflow, /--draft=false --prerelease --latest=false/);
  assert.match(workflow, /isDraft,isPrerelease/);
  assert.doesNotMatch(workflow, /git fetch --force/);
});
