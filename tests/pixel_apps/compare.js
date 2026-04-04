#!/usr/bin/env node
/**
 * Pixel comparison between different rendering outputs.
 *
 * Compares:
 * 1. pipeline (load_html) vs web (Playwright) — validates our rendering engine
 * 2. framework (view! macro) vs pipeline — validates framework DOM matches HTML parser
 * 3. framework (view! macro) vs web — the ultimate end-to-end test
 */

const fs = require('fs');
const path = require('path');
const { PNG } = require('pngjs');
const pixelmatch = require('pixelmatch').default;

function compareDirectories(refDir, testDir, diffDir, label, threshold) {
    fs.mkdirSync(diffDir, { recursive: true });

    const refFiles = fs.readdirSync(refDir)
        .filter(f => f.endsWith('.png'))
        .sort();

    console.log(`\n${label}`);
    console.log(`  Reference: ${refDir}`);
    console.log(`  Test:      ${testDir}`);
    console.log(`  Threshold: ${threshold}\n`);
    console.log('  ' + 'App'.padEnd(30) + 'Pixels'.padEnd(12) + 'Diff'.padEnd(12) + 'Match %');
    console.log('  ' + '-'.repeat(66));

    let allPass = true;
    let totalDiff = 0;
    let totalPixels = 0;

    for (const file of refFiles) {
        const refPath = path.join(refDir, file);
        const testPath = path.join(testDir, file);

        if (!fs.existsSync(testPath)) {
            console.log(`  ${file.replace('.png','').padEnd(30)}MISSING`);
            allPass = false;
            continue;
        }

        const refImg = PNG.sync.read(fs.readFileSync(refPath));
        const testImg = PNG.sync.read(fs.readFileSync(testPath));

        if (refImg.width !== testImg.width || refImg.height !== testImg.height) {
            console.log(
                `  ${file.replace('.png','').padEnd(30)}SIZE MISMATCH: ` +
                `ref=${refImg.width}x${refImg.height} ` +
                `test=${testImg.width}x${testImg.height}`
            );
            allPass = false;
            continue;
        }

        const { width, height } = refImg;
        const diff = new PNG({ width, height });

        const numDiffPixels = pixelmatch(
            refImg.data, testImg.data, diff.data,
            width, height, { threshold }
        );

        const total = width * height;
        const matchPct = (100 - (numDiffPixels / total) * 100).toFixed(4);
        const name = file.replace('.png', '');

        console.log(
            `  ${name.padEnd(30)}${total.toString().padEnd(12)}` +
            `${numDiffPixels.toString().padEnd(12)}${matchPct}%`
        );

        fs.writeFileSync(path.join(diffDir, file), PNG.sync.write(diff));

        totalDiff += numDiffPixels;
        totalPixels += total;
        if (numDiffPixels > 0) allPass = false;
    }

    const overallMatch = totalPixels > 0
        ? (100 - (totalDiff / totalPixels) * 100).toFixed(4)
        : '100.0000';

    console.log('  ' + '-'.repeat(66));
    console.log(`  Overall: ${overallMatch}% match (${totalDiff} diff pixels / ${totalPixels} total)\n`);

    return { allPass, overallMatch, totalDiff };
}

const baseDir = __dirname;
const webDir = path.join(baseDir, 'screenshots', 'web');
const pipelineDir = path.join(baseDir, 'screenshots', 'rust');
const frameworkDir = path.join(baseDir, 'screenshots', 'framework');

console.log('═══════════════════════════════════════════════════════════════');
console.log('                  Open UI Pixel Comparison');
console.log('═══════════════════════════════════════════════════════════════');

// Comparison 1: Pipeline (load_html) vs Web (Playwright headless_shell)
const pipelineResult = compareDirectories(
    webDir, pipelineDir,
    path.join(baseDir, 'screenshots', 'diff-pipeline'),
    '📊 Pipeline (load_html) vs Web (headless_shell)',
    0.15
);

const hasFramework = fs.existsSync(frameworkDir) &&
    fs.readdirSync(frameworkDir).filter(f => f.endsWith('.png')).length > 0;

if (hasFramework) {
    // Comparison 2: Framework (view! macro) vs Pipeline (load_html)
    const frameworkResult = compareDirectories(
        pipelineDir, frameworkDir,
        path.join(baseDir, 'screenshots', 'diff-framework'),
        '🧩 Framework (view! macro) vs Pipeline (load_html)',
        0.0
    );

    // Comparison 3: Framework vs Web — the ultimate end-to-end test
    const directResult = compareDirectories(
        webDir, frameworkDir,
        path.join(baseDir, 'screenshots', 'diff-direct'),
        '🎯 Framework (view! macro) vs Web (headless_shell)',
        0.15
    );

    console.log('═══════════════════════════════════════════════════════════════');
    console.log('                        Summary');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log(`  Pipeline vs Web:       ${pipelineResult.overallMatch}%`);
    console.log(`  Framework vs Pipeline: ${frameworkResult.overallMatch}%`);
    console.log(`  Framework vs Web:      ${directResult.overallMatch}%`);
    console.log('═══════════════════════════════════════════════════════════════\n');

    if (frameworkResult.allPass) {
        console.log('✅ Framework produces PIXEL-PERFECT output matching the pipeline!');
    }
    if (parseFloat(directResult.overallMatch) >= 99.0) {
        console.log('✅ Framework matches real Chromium within 99%+ (text anti-aliasing only).');
    } else if (parseFloat(directResult.overallMatch) >= 97.0) {
        console.log('⚠️  Framework matches real Chromium within 97%+ range.');
    }
    if (parseFloat(pipelineResult.overallMatch) >= 97.0) {
        console.log('✅ Pipeline rendering matches real Chromium within acceptable range.');
    }

    // Exit non-zero if any comparison fell below its threshold or had
    // individual file failures (missing files, size mismatches).
    const pipelineOk = parseFloat(pipelineResult.overallMatch) >= 97.0 && pipelineResult.allPass;
    const frameworkOk = parseFloat(directResult.overallMatch) >= 95.0 && directResult.allPass;
    if (!pipelineOk || !frameworkOk) {
        console.log('\n❌ FAIL: pixel comparison thresholds not met or individual files failed.');
        process.exit(1);
    }
} else {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log(`Pipeline vs Web: ${pipelineResult.overallMatch}% match`);
    console.log('(Framework screenshots not yet generated)');
    console.log('═══════════════════════════════════════════════════════════════');

    if (parseFloat(pipelineResult.overallMatch) < 97.0 || !pipelineResult.allPass) {
        console.log('\n❌ FAIL: pipeline comparison below 97% threshold or individual files failed.');
        process.exit(1);
    }
}
