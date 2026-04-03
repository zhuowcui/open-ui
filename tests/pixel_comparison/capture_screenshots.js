// capture_screenshots.js — Uses Playwright to screenshot HTML test pages.
// Each page is loaded in a headless Chromium browser at 800x600 viewport
// and saved as a PNG for later pixel comparison with Open UI renders.

const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

const HTML_DIR = path.join(__dirname, 'html_pages');
const SCREENSHOT_DIR = path.join(__dirname, 'screenshots');

async function main() {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

  // Use our own M147 headless_shell to ensure identical Blink/Skia/FreeType
  // rendering between Playwright screenshots and Open UI's DummyPageHolder.
  const homedir = require('os').homedir();
  const execPath = path.join(homedir, 'chromium/src/out/Release/headless_shell');
  const browser = await chromium.launch({
    headless: true,
    executablePath: execPath,
    args: [
      '--font-render-hinting=none',
      '--disable-lcd-text',
      '--disable-font-subpixel-positioning',
      '--force-color-profile=srgb',
    ],
  });
  const context = await browser.newContext({
    viewport: { width: 800, height: 600 },
    deviceScaleFactor: 1,
  });

  const htmlFiles = fs.readdirSync(HTML_DIR)
    .filter(f => f.endsWith('.html'))
    .sort();

  console.log(`Found ${htmlFiles.length} HTML test pages`);

  for (const file of htmlFiles) {
    const page = await context.newPage();
    const filePath = path.join(HTML_DIR, file);
    await page.goto(`file://${filePath}`);
    // Wait for layout to settle.
    await page.waitForTimeout(200);

    const screenshotPath = path.join(SCREENSHOT_DIR, file.replace('.html', '.png'));
    await page.screenshot({ path: screenshotPath, fullPage: false });
    console.log(`  Captured: ${file} -> ${path.basename(screenshotPath)}`);
    await page.close();
  }

  await browser.close();
  console.log(`\nAll ${htmlFiles.length} screenshots saved to ${SCREENSHOT_DIR}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
