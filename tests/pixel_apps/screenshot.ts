import { chromium } from 'playwright';
import * as path from 'path';
import * as fs from 'fs';

async function main() {
    const appsDir = path.join(__dirname, 'apps');
    const outDir = path.join(__dirname, 'screenshots', 'web');
    fs.mkdirSync(outDir, { recursive: true });

    // Use our own Chromium build so font rendering matches our pipeline exactly.
    // Both the reference screenshots and our Rust framework use the same Blink
    // engine with identical font settings.
    const chromiumSrc = process.env.CHROMIUM_SRC ||
        path.join(process.env.HOME!, 'chromium', 'src');
    const headlessShell = path.join(chromiumSrc, 'out', 'Release', 'headless_shell');

    const browser = await chromium.launch({
        headless: true,
        executablePath: headlessShell,
        args: [
            '--no-sandbox',
            '--font-render-hinting=none',
            '--disable-lcd-text',
            '--disable-font-subpixel-positioning',
        ],
    });
    const context = await browser.newContext({
        viewport: { width: 1200, height: 800 },
        deviceScaleFactor: 1,
    });

    const files = fs.readdirSync(appsDir)
        .filter(f => f.endsWith('.html'))
        .sort();

    for (const file of files) {
        const filePath = path.join(appsDir, file);
        const page = await context.newPage();
        await page.goto(`file://${filePath}`);
        await page.waitForTimeout(500);

        const name = file.replace('.html', '');
        await page.screenshot({
            path: path.join(outDir, `${name}.png`),
            fullPage: false,
        });

        console.log(`Screenshotted: ${name}`);
        await page.close();
    }

    await browser.close();
    console.log(`Done! ${files.length} screenshots saved to ${outDir}`);
}

main().catch(console.error);
