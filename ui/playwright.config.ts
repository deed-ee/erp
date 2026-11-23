import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  use: { baseURL: 'http://127.0.0.1:4200' },
  webServer: {
    command: 'npm start',
    url: 'http://127.0.0.1:4200',
    reuseExistingServer: !process.env.CI
  }
});
