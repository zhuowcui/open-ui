// compare_pixels.js — Compares Playwright screenshots with Open UI renders.
// Loads both PNGs, compares pixel-by-pixel, and reports diff percentage.
// Uses a configurable tolerance for per-channel comparison.
//
// Usage: node compare_pixels.js <screenshots_dir> <openui_renders_dir> [tolerance]

const fs = require('fs');
const path = require('path');
const { PNG } = require('pngjs');

const TOLERANCE = parseInt(process.argv[4] || '2', 10);
const screenshotDir = process.argv[2];
const renderDir = process.argv[3];

if (!screenshotDir || !renderDir) {
  console.error('Usage: node compare_pixels.js <screenshots_dir> <openui_renders_dir> [tolerance]');
  process.exit(1);
}

function loadPNG(filePath) {
  const data = fs.readFileSync(filePath);
  return PNG.sync.read(data);
}

function compareImages(img1, img2, tolerance) {
  if (img1.width !== img2.width || img1.height !== img2.height) {
    return {
      match: false,
      reason: `Size mismatch: ${img1.width}x${img1.height} vs ${img2.width}x${img2.height}`,
      diffPercent: 100,
      maxDiff: 255,
    };
  }

  const totalPixels = img1.width * img1.height;
  let diffPixels = 0;
  let maxChannelDiff = 0;

  for (let i = 0; i < img1.data.length; i += 4) {
    const dr = Math.abs(img1.data[i] - img2.data[i]);
    const dg = Math.abs(img1.data[i + 1] - img2.data[i + 1]);
    const db = Math.abs(img1.data[i + 2] - img2.data[i + 2]);
    const da = Math.abs(img1.data[i + 3] - img2.data[i + 3]);

    const maxD = Math.max(dr, dg, db, da);
    maxChannelDiff = Math.max(maxChannelDiff, maxD);

    if (maxD > tolerance) {
      diffPixels++;
    }
  }

  const diffPercent = (diffPixels / totalPixels) * 100;
  return {
    match: diffPixels === 0,
    diffPercent: diffPercent.toFixed(4),
    maxDiff: maxChannelDiff,
    diffPixels,
    totalPixels,
  };
}

function main() {
  const screenshotFiles = fs.readdirSync(screenshotDir)
    .filter(f => f.endsWith('.png'))
    .sort();

  console.log(`Comparing ${screenshotFiles.length} image pairs (tolerance=${TOLERANCE})\n`);

  let allPass = true;
  const results = [];

  for (const file of screenshotFiles) {
    const screenshotPath = path.join(screenshotDir, file);
    const renderPath = path.join(renderDir, file);

    if (!fs.existsSync(renderPath)) {
      console.log(`  FAIL: ${file} — no Open UI render found`);
      allPass = false;
      results.push({ file, status: 'FAIL', reason: 'Missing render', diffPercent: 100, diffPixels: 0, totalPixels: 0, maxDiff: 0 });
      continue;
    }

    const screenshot = loadPNG(screenshotPath);
    const render = loadPNG(renderPath);
    const result = compareImages(screenshot, render, TOLERANCE);

    const status = result.match ? 'PASS' : (parseFloat(result.diffPercent) < 1.0 ? 'WARN' : 'FAIL');
    if (status === 'FAIL') allPass = false;

    console.log(`  ${status}: ${file}`);
    console.log(`         Diff: ${result.diffPercent}% (${result.diffPixels}/${result.totalPixels} pixels)`);
    console.log(`         Max channel diff: ${result.maxDiff}`);
    if (result.reason) console.log(`         Reason: ${result.reason}`);
    console.log('');

    results.push({ file, ...result, status });
  }

  // Summary
  const passed = results.filter(r => r.status === 'PASS').length;
  const warned = results.filter(r => r.status === 'WARN').length;
  const failed = results.filter(r => r.status === 'FAIL').length;

  console.log('─────────────────────────────────────────');
  console.log(`Results: ${passed} PASS, ${warned} WARN, ${failed} FAIL out of ${results.length} pages`);

  if (!allPass) {
    console.log('\nSome pages have significant pixel differences.');
    process.exit(1);
  } else {
    console.log('\nAll pages are pixel-perfect (within tolerance)!');
  }
}

main();
