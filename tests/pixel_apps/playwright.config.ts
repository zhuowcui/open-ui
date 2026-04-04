import { defineConfig } from '@playwright/test';

export default defineConfig({
  use: {
    viewport: { width: 1200, height: 800 },
    deviceScaleFactor: 1,
    headless: true,
  },
});
