import { test, expect, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';

test.use({ ignoreHTTPSErrors: true }); // Only for the locally generated preview certificate.

async function login(page: Page, user: 'production' | 'auditor') {
  const passwords: Record<string, string> = JSON.parse(readFileSync('../.erp4us-preview/credentials.json', 'utf8'));
  await page.goto('/local-access');
  await page.getByRole('button', { name: 'Sign in with Keycloak' }).click();
  await page.waitForURL('https://127.0.0.1:9443/**');
  await page.locator('#username').fill(user);
  await page.locator('#password').fill(passwords[user]);
  await page.locator('#kc-login').click();
  await expect(page.getByLabel('Company and organization')).toBeVisible({ timeout: 20000 });
  await expect(page).toHaveURL('http://127.0.0.1:4200/');
}

test('real OIDC: production planner calculates the NovaMotion bill of materials', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  await login(page, 'production');
  await page.getByRole('link', { name: 'Manufacturing' }).click();
  await page.getByRole('button', { name: 'Calculate gross needs' }).click();
  await expect(page.getByRole('row').filter({ hasText: 'NM-ENDPLATE' })).toContainText('20 EA');
});

test('real OIDC: the other tenant cannot read NovaMotion manufacturing data', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  // Read the actual access token from the session-call request in memory. Send
  // it back to the owning service with explicitly unrelated scope headers.
  const session = page.waitForRequest(request => request.url().endsWith('/v1/session'));
  await login(page, 'auditor');
  const authorization = (await (await session).allHeaders()).authorization;
  expect(await page.getByRole('link', { name: 'Manufacturing' }).count()).toBe(0);
  const result = await page.request.get('/v1/manufacturing/products', { headers: {
    Authorization: authorization,
    'X-Erp4us-Tenant': 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5',
    'X-Erp4us-Organization': '3e793fb5-75a7-45f6-8f66-45a58fcec674',
  }});
  expect(result.status()).toBe(403);
});
