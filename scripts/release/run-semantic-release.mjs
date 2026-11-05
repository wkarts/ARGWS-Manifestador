import fs from 'node:fs';
import semanticRelease from 'semantic-release';

const result = await semanticRelease({}, {
  cwd: process.cwd(),
  env: process.env,
  stdout: process.stdout,
  stderr: process.stderr,
});

const outputLines = [];
if (result?.nextRelease) {
  outputLines.push('published=true');
  outputLines.push(`version=${result.nextRelease.version}`);
  outputLines.push(`tag=${result.nextRelease.gitTag}`);
  outputLines.push(`name=${result.nextRelease.name ?? result.nextRelease.gitTag}`);
  outputLines.push(`notes_b64=${Buffer.from(result.nextRelease.notes ?? '', 'utf8').toString('base64')}`);
} else {
  outputLines.push('published=false');
  outputLines.push('version=');
  outputLines.push('tag=');
  outputLines.push('name=');
  outputLines.push('notes_b64=');
}

if (process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `${outputLines.join('\n')}\n`);
} else {
  console.log(outputLines.join('\n'));
}
