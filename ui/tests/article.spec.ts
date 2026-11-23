import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { randomUUID } from 'node:crypto';

test('keyboard registration, duplicate recovery and retrieval against the local service', async ({ page }) => {
  // Read only the demo bearer, never include credentials in test reports.
  const token = /^ERP4US_DEMO_TOKEN=(.+)$/m.exec(readFileSync('../.env', 'utf8'))?.[1];
  if (!token) throw new Error('Initialize the local demo before running the browser scenario.');
  await page.goto('/');
  for (const button of await page.getByRole('button').all()) {
    await expect(button).toHaveAccessibleName(/.+/);
    const icon = button.locator('svg[aria-hidden="true"]');
    await expect(icon).toBeVisible();
    // A horizontal stroked SVG path has zero bounding-box height on its own.
    await expect(icon.locator('path[d]').first()).toBeAttached();
  }
  await page.getByRole('button', {name: 'English'}).press('Enter');
  await page.getByLabel('Demo token').fill(token);
  const register = page.getByRole('button', {name: 'Register article'});
  await register.press('Enter');
  await expect(page.getByRole('status')).toHaveText('Check the SKU and name.');
  const sku = `UI-${randomUUID().slice(0,8).toUpperCase()}`;
  await page.getByLabel('SKU', {exact: true}).fill(sku);
  await page.getByLabel('Name', {exact: true}).fill('Steel bracket');
  await register.press('Enter');
  await expect(page.getByRole('status')).toHaveText('Article registered.');
  const id = await page.getByLabel('Article ID').inputValue();
  await register.press('Enter');
  await expect(page.getByRole('status')).toHaveText(/SKU already exists/);
  await expect(page.getByLabel('Name', {exact: true})).toHaveValue('Steel bracket');
  await page.reload();
  await expect(page.getByLabel('Demo-Token')).toBeEmpty();
  await page.getByLabel('Artikel-ID').fill(id);
  await page.getByLabel('Demo-Token').fill(token);
  await page.getByRole('button', {name: 'Artikel abrufen'}).press('Enter');
  await expect(page.getByRole('status')).toHaveText('Artikel abgerufen.');
  await expect(page.getByRole('definition').filter({hasText: sku})).toBeVisible();
});
