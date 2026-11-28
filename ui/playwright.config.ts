import { defineConfig } from '@playwright/test';

const port = process.env.ERP4US_UI_PORT || '4200';
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: './tests',
  use: { baseURL, locale: 'en-US' },
  webServer: {
    command: `npm start -- --port ${Number(port)}`,
    url: baseURL,
    reuseExistingServer: !process.env.CI
  }
});
