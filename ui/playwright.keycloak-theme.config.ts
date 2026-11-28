import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './identity-tests',
  outputDir: 'test-results/keycloak-theme',
  workers: 1,
  use: { ignoreHTTPSErrors: true, locale: 'en-US' } // Disposable loopback certificate only.
});
