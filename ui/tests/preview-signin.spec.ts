import { test, expect, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';

test.use({ ignoreHTTPSErrors: true }); // Only for the locally generated preview certificate.

async function login(page: Page, user: 'production' | 'auditor' | 'platform-monitor') {
  const passwords: Record<string, string> = JSON.parse(readFileSync('../.erp4us-preview/credentials.json', 'utf8'));
  await page.goto('/local-access');
  await page.getByRole('button', { name: 'Sign in with Keycloak' }).click();
  await page.waitForURL('https://127.0.0.1:9443/**');
  await page.locator('#username').fill(user);
  await page.locator('#password').fill(passwords[user]);
  await page.locator('#kc-login').click();
  if (user === 'platform-monitor') {
    await expect(page.getByRole('banner')).toContainText('Platform operations', { timeout: 20000 });
  } else {
    await expect(page.getByLabel('Company and organization')).toBeVisible({ timeout: 20000 });
  }
  await expect(page).toHaveURL('http://127.0.0.1:4200/');
}

test('real OIDC: production planner calculates the NovaMotion bill of materials', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  await login(page, 'production');
  await page.getByRole('link', { name: 'Manufacturing' }).click();
  await page.getByRole('button', { name: 'Calculate gross needs' }).click();
  await expect(page.getByRole('row').filter({ hasText: 'NM-ENDPLATE' })).toContainText('20 EA');
  await page.getByRole('link', { name: 'Master data', exact: true }).click();
  await page.getByRole('link', { name: 'Business partners' }).click();
  await page.getByLabel('Role', { exact: true }).selectOption('customer');
  await expect(page.getByRole('row').filter({ hasText: 'Atlas Handling Technologies GmbH' })).toBeVisible();
  await expect(page.getByRole('row').filter({ hasText: 'VectorDrive' })).toHaveCount(0);
  await page.getByLabel('Role', { exact: true }).selectOption('supplier');
  await expect(page.getByRole('row').filter({ hasText: 'VectorDrive' })).toBeVisible();
});

test('real OIDC: the other tenant cannot read NovaMotion manufacturing data', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  // Read the actual access token from the session-call request in memory. Send
  // it back to the owning service with explicitly unrelated scope headers.
  const session = page.waitForRequest(request => request.url().endsWith('/v1/session'));
  await login(page, 'auditor');
  const authorization = (await (await session).allHeaders()).authorization;
  expect(await page.getByRole('link', { name: 'Manufacturing' }).count()).toBe(0);
  expect(await page.getByRole('link', { name: 'Sales orders' }).count()).toBe(0);
  const result = await page.request.get('/v1/manufacturing/products', { headers: {
    Authorization: authorization,
    'X-Erp4us-Tenant': 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5',
    'X-Erp4us-Organization': '3e793fb5-75a7-45f6-8f66-45a58fcec674',
  }});
  expect(result.status()).toBe(403);
  expect((await page.request.get('/v1/admin/application-health', { headers: { Authorization: authorization } })).status()).toBe(403);
});

test('real OIDC: platform monitor sees actual application health', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  await login(page, 'platform-monitor');
  await page.getByRole('link', { name: 'Administration' }).click();
  await expect(page.getByRole('heading', { name: 'Application health' })).toBeVisible();
  await expect(page.getByRole('row').filter({ hasText: 'master-data' }).getByRole('cell', { name: 'Ready', exact: true })).toBeVisible();
  await expect(page.getByRole('row').filter({ hasText: 'manufacturing' }).getByRole('cell', { name: 'Ready', exact: true })).toBeVisible();
  await expect(page.getByRole('row').filter({ hasText: 'jetstream' }).getByRole('cell', { name: 'Ready', exact: true })).toBeVisible();
  await expect(page.getByText('Stored messages', { exact: true })).toBeVisible();
  await expect(page.getByText('Awaiting acknowledgement', { exact: true })).toBeVisible();
});

test('real OIDC: production user saves and reopens a priced sales draft', async ({ page }) => {
  test.skip(process.env.ERP4US_REAL_OIDC_E2E !== '1', 'Requires the dedicated Keycloak preview stack');
  await login(page, 'production');
  await page.getByRole('link', { name: 'Sales orders' }).click();
  await expect(page.getByRole('option', { name: /CUS-ATLAS/ })).toHaveCount(1);
  await page.getByLabel('Customer', { exact: true }).selectOption({ label: 'CUS-ATLAS · Atlas Handling Technologies GmbH' });
  await expect(page.getByRole('option', { name: /NM-1000/ })).toHaveCount(1);
  await page.getByLabel('Pieces', { exact: true }).fill('10');
  const saved = page.waitForResponse(response => response.url().endsWith('/v1/sales/orders')
    && response.request().method() === 'POST');
  await page.getByRole('button', { name: 'Save draft' }).click();
  expect((await saved).status()).toBe(201);
  await expect(page.getByText('Draft saved: EUR 24900.00')).toBeVisible();
  await page.getByRole('link', { name: 'Manufacturing' }).click();
  await page.getByRole('link', { name: 'Sales orders' }).click();
  const match = page.getByRole('row').filter({ hasText: 'Atlas Handling Technologies GmbH' })
    .filter({ hasText: 'NM-1000' }).filter({ hasText: 'EUR 24900.00' });
  await expect(match.first()).toContainText('10 EA');
});
