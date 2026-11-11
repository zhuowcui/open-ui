// Copies the fonts and comparison images the site uses from this
// repository, unmodified, and measures each comparison.
//
//   node scripts/sync-assets.mjs           copy, measure, write the summary
//   node scripts/sync-assets.mjs --check   verify only; write nothing
//
// Every source is checked against the SHA-256 in scripts/assets.json, and
// each pair's differing-pixel count must equal the count its record states.
// Nothing here edits an image: the files are copied byte for byte, and the
// diff is computed from decoded RGBA without writing a new image.

import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = resolve(siteRoot, '..');
const checkOnly = process.argv.includes('--check');
const manifest = JSON.parse(readFileSync(join(siteRoot, 'scripts/assets.json'), 'utf8'));

const failures = [];

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

/** Reads a repository file and checks its digest. */
function readVerified(entry) {
  const bytes = readFileSync(join(repoRoot, entry.source));
  const digest = sha256(bytes);
  if (digest !== entry.sha256) {
    failures.push(`${entry.source}: SHA-256 ${digest}, expected ${entry.sha256}`);
  }
  return bytes;
}

function copy(entry) {
  if (checkOnly) return;
  const to = join(siteRoot, entry.dest);
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(join(repoRoot, entry.source), to);
}

/**
 * Decodes a non-interlaced 8-bit RGB or RGBA PNG to RGBA. That is what the
 * capture tools write; anything else is rejected rather than guessed.
 */
function decodePng(bytes, name) {
  const signature = '89504e470d0a1a0a';
  if (bytes.subarray(0, 8).toString('hex') !== signature) throw new Error(`${name}: not a PNG`);
  let offset = 8;
  let header;
  const data = [];
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.toString('latin1', offset + 4, offset + 8);
    const body = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === 'IHDR') {
      header = {
        width: body.readUInt32BE(0),
        height: body.readUInt32BE(4),
        depth: body[8],
        colorType: body[9],
        interlace: body[12],
      };
    } else if (type === 'IDAT') {
      data.push(body);
    } else if (type === 'IEND') {
      break;
    }
    offset += 12 + length;
  }
  if (!header) throw new Error(`${name}: no IHDR`);
  const channels = { 2: 3, 6: 4 }[header.colorType];
  if (header.depth !== 8 || !channels || header.interlace !== 0) {
    throw new Error(
      `${name}: unsupported PNG (depth ${header.depth}, color type ${header.colorType}, interlace ${header.interlace})`,
    );
  }
  const { width, height } = header;
  const raw = inflateSync(Buffer.concat(data));
  const stride = width * channels;
  const out = new Uint8Array(width * height * 4);
  let previous = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const current = new Uint8Array(stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= channels ? current[i - channels] : 0;
      const b = previous[i];
      const c = i >= channels ? previous[i - channels] : 0;
      let predictor = 0;
      if (filter === 1) predictor = a;
      else if (filter === 2) predictor = b;
      else if (filter === 3) predictor = (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        predictor = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (filter !== 0) throw new Error(`${name}: bad filter ${filter}`);
      current[i] = (line[i] + predictor) & 0xff;
    }
    for (let x = 0; x < width; x++) {
      const o = (y * width + x) * 4;
      out[o] = current[x * channels];
      out[o + 1] = current[x * channels + 1];
      out[o + 2] = current[x * channels + 2];
      out[o + 3] = channels === 4 ? current[x * channels + 3] : 255;
    }
    previous = current;
  }
  return { width, height, rgba: out };
}

/** Counts pixels whose RGBA bytes differ, with their bounds and largest channel delta. */
function compare(a, b) {
  if (a.width !== b.width || a.height !== b.height) {
    throw new Error(`sizes differ: ${a.width}x${a.height} and ${b.width}x${b.height}`);
  }
  let count = 0;
  let maxDelta = 0;
  let bounds = null;
  for (let y = 0; y < a.height; y++) {
    for (let x = 0; x < a.width; x++) {
      const o = (y * a.width + x) * 4;
      let delta = 0;
      for (let c = 0; c < 4; c++) delta = Math.max(delta, Math.abs(a.rgba[o + c] - b.rgba[o + c]));
      if (delta === 0) continue;
      count++;
      maxDelta = Math.max(maxDelta, delta);
      if (!bounds) bounds = { x0: x, y0: y, x1: x, y1: y };
      bounds.x0 = Math.min(bounds.x0, x);
      bounds.y0 = Math.min(bounds.y0, y);
      bounds.x1 = Math.max(bounds.x1, x);
      bounds.y1 = Math.max(bounds.y1, y);
    }
  }
  return {
    width: a.width,
    height: a.height,
    diffPixels: count,
    maxChannelDelta: maxDelta,
    bounds: bounds && {
      x: bounds.x0,
      y: bounds.y0,
      width: bounds.x1 - bounds.x0 + 1,
      height: bounds.y1 - bounds.y0 + 1,
    },
  };
}

for (const font of manifest.fonts) {
  readVerified(font);
  copy(font);
}

const { test, crop, cases } = manifest.comparisons;
const summary = { test, crop, cases: [] };
for (const item of cases) {
  const chromiumBytes = readVerified(item.chromium);
  const openuiBytes = readVerified(item.openui);
  const result = compare(
    decodePng(chromiumBytes, item.chromium.source),
    decodePng(openuiBytes, item.openui.source),
  );
  if (result.diffPixels !== item.expectedDiffPixels) {
    failures.push(
      `${item.id}: ${result.diffPixels} differing pixels, the record states ${item.expectedDiffPixels}`,
    );
  }
  copy(item.chromium);
  copy(item.openui);
  const publicPath = (dest) => dest.replace(/^public\//, '');
  summary.cases.push({
    id: item.id,
    label: item.label,
    summary: item.summary,
    measured: item.measured,
    evidence: item.evidence,
    chromium: { src: publicPath(item.chromium.dest), source: item.chromium.source, sha256: item.chromium.sha256 },
    openui: { src: publicPath(item.openui.dest), source: item.openui.source, sha256: item.openui.sha256 },
    ...result,
  });
  console.log(
    `${item.id.padEnd(7)} ${result.width}x${result.height}  ${result.diffPixels} differing pixels` +
      (result.diffPixels ? `, max channel delta ${result.maxChannelDelta}` : ''),
  );
}

if (failures.length) {
  console.error(`\n${failures.length} asset check(s) failed:\n  ${failures.join('\n  ')}`);
  process.exit(1);
}

if (!checkOnly) {
  const out = join(siteRoot, 'src/assets/generated/comparisons.json');
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, `${JSON.stringify(summary, null, 2)}\n`);
}
console.log(checkOnly ? 'All assets verified.' : 'Assets copied and verified.');
