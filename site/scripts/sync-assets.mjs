// Copies the fonts and the Div brand assets the site uses from this
// repository, unmodified.
//
//   node scripts/sync-assets.mjs           copy
//   node scripts/sync-assets.mjs --check   verify only; write nothing
//
// Every source is checked against the SHA-256 in scripts/assets.json first.
// The app screenshots are not copied: they are rendered by
// site/showcase and committed in src/assets/showcase/.

import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(siteRoot, '..');
const checkOnly = process.argv.includes('--check');
const manifest = JSON.parse(readFileSync(join(siteRoot, 'scripts/assets.json'), 'utf8'));

const failures = [];
for (const font of [...manifest.fonts, ...manifest.brand]) {
  const bytes = readFileSync(join(repoRoot, font.source));
  const digest = createHash('sha256').update(bytes).digest('hex');
  if (digest !== font.sha256) {
    failures.push(`${font.source}: SHA-256 ${digest}, expected ${font.sha256}`);
    continue;
  }
  if (!checkOnly) {
    const dest = join(siteRoot, font.dest);
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(join(repoRoot, font.source), dest);
  }
}

if (failures.length) {
  console.error(`${failures.length} asset check(s) failed:\n  ${failures.join('\n  ')}`);
  process.exit(1);
}
console.log(checkOnly ? 'All assets verified.' : 'Assets copied and verified.');
