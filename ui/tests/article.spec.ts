import { test, expect, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';

async function setDemoAccess(page: Page, token: string) {
  await page.getByRole('link', { name: 'Local demo access' }).click();
  await page.getByLabel('Demo token').fill(token);
  await page.getByRole('button', { name: 'Use for this session' }).press('Enter');
  await expect(page.getByLabel('Demo token')).toBeEmpty();
}

test('real API: keyboard registration, duplicate recovery and retrieval after reload', async ({ page }) => {
  const token = execFileSync('python3', ['tests/integration/identity_fixture.py', 'token'], {
    cwd: '..', encoding: 'utf8',
  }).trim();
  await page.goto('/');
  await setDemoAccess(page, token);
  await page.getByRole('link', { name: 'Open products' }).click();
  await page.getByRole('link', { name: 'Register product' }).press('Enter');
  const save = page.getByRole('button', { name: 'Save product' });
  await save.press('Enter');
  await expect(page.getByRole('status')).toHaveText('Check the SKU and name.');
  const skuInput = page.getByLabel('SKU', { exact: true });
  const nameInput = page.getByLabel('Name', { exact: true });
  await expect(skuInput).toBeFocused();
  const sku = `UI-${randomUUID().slice(0, 8).toUpperCase()}`;
  await skuInput.fill(sku);
  await nameInput.fill('Steel bracket');
  await save.press('Enter');
  await expect(page.getByRole('status')).toHaveText('Article registered.');
  await expect(page.getByRole('definition').filter({ hasText: sku })).toBeVisible();
  const detailUrl = page.url();
  const id = new URL(detailUrl).pathname.split('/').at(-1)!;

  await page.goto('/products/new');
  await setDemoAccess(page, token);
  await page.getByRole('link', { name: 'Open products' }).click();
  await page.getByRole('link', { name: 'Register product' }).click();
  await skuInput.fill(sku);
  await nameInput.fill('Steel bracket');
  await save.press('Enter');
  await expect(page.getByRole('status')).toHaveText(/SKU already exists/);
  await expect(skuInput).toBeFocused();
  await expect(nameInput).toHaveValue('Steel bracket');

  await page.goto(detailUrl);
  await expect(page.getByRole('status')).toHaveText('Set a local demo credential first.');
  await page.getByRole('banner').getByRole('link', { name: 'Local demo access' }).click();
  await expect(page.getByLabel('Demo token')).toBeEmpty();
  await page.getByLabel('Demo token').fill(token);
  await page.getByRole('button', { name: 'Use for this session' }).press('Enter');
  await page.getByRole('link', { name: 'Open products' }).click();
  await page.getByLabel('Article ID').fill(id);
  await page.getByRole('button', { name: 'Retrieve article' }).press('Enter');
  await expect(page.getByRole('status')).toHaveText('Article retrieved.');
  await expect(page.getByRole('definition').filter({ hasText: sku })).toBeVisible();
});

test('lookup displays pending and API error states without inventing a record', async ({ page }) => {
  await page.goto('/');
  await setDemoAccess(page, 'playwright-fixture-credential');
  let release!: () => void;
  const response = new Promise<void>(resolve => { release = resolve; });
  await page.route('**/v1/articles/*', async route => {
    await response;
    await route.fulfill({ status: 503 });
  });
  await page.getByRole('link', { name: 'Open products' }).click();
  await page.getByRole('button', { name: 'Retrieve article' }).click();
  await expect(page.getByLabel('Article ID')).toBeFocused();
  await expect(page.getByRole('status')).toHaveText(/valid article ID/);
  await page.getByLabel('Article ID').fill(randomUUID());
  await page.getByRole('button', { name: 'Retrieve article' }).click();
  await expect(page.getByRole('status')).toHaveText('Request in progress …');
  release();
  await expect(page.getByRole('status')).toHaveText(/Request failed/);
  await expect(page.getByRole('definition')).toHaveCount(0);
});
