import { randomUUID } from 'node:crypto';
import { expect, test, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { demoToken, setDemoAccess } from './demo-access';

const tenant = 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5';
const organization = '3e793fb5-75a7-45f6-8f66-45a58fcec674';
const otherOrganization = '8af68084-19d6-4e31-8203-f9f4708b9bee';
const otherTenant = 'a260610d-ebcf-4bb7-b871-9d221ace30a8';
const externalOrganization = '2886c61e-85d8-4688-813a-b59c6641f9f8';

async function seed(page: Page, subject: string, tenantId: string, organizationId: string) {
  const record = { sku: `FIND-${randomUUID().slice(0, 8).toUpperCase()}`, name: 'Synthetic service kit' };
  const response = await page.request.post('/v1/articles:register', {
    data: record,
    headers: {
      Authorization: `Bearer ${demoToken(subject)}`, 'X-Erp4us-Tenant': tenantId,
      'X-Erp4us-Organization': organizationId
    }
  });
  expect(response.status()).toBe(201);
  const article: { id: string } = await response.json();
  return { ...record, id: article.id };
}

test('live scoped lookup in English and German, with a direct route to the same article', async ({ page }) => {
  const own = await seed(page, 'synthetic-writer', tenant, organization);
  const foreignStock = [
    await seed(page, 'synthetic-registration', tenant, otherOrganization),
    await seed(page, 'synthetic-registration', otherTenant, externalOrganization)
  ];
  const requests: string[] = [];
  page.on('request', request => {
    if (new URL(request.url()).pathname.startsWith('/v1/')) requests.push(request.method());
  });
  await page.goto('/');
  await setDemoAccess(page, demoToken('synthetic-reader'));
  await page.getByRole('link', { name: 'Open products' }).click();
  await page.keyboard.press('Control+k');
  const query = page.getByRole('searchbox', { name: 'Global search' });
  await expect(query).toBeFocused();
  const suggestion = page.getByRole('button', { name: 'Find article by SKU' });
  await query.fill('find art');
  await expect(suggestion).toBeVisible();
  await suggestion.press('Enter');
  await expect(query).toBeFocused();
  await expect(query).toHaveValue('find article with sku ');
  await query.fill(`Show me the article with SKU ${own.sku.toLowerCase()}?`);
  await query.press('Enter');
  const answer = page.getByRole('region', { name: 'Search results' });
  await expect(answer.getByRole('status')).toHaveText('One matching article');
  await expect(answer.getByRole('heading', { name: own.name })).toBeVisible();
  await expect(answer).toContainText(own.sku);
  await expect(answer).toContainText('Interpreted as');
  await expect(answer).toContainText('Fixed demo scope');
  await expect(answer).toContainText('Master data');
  const scan = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']).analyze();
  expect(scan.violations).toEqual([]);
  await answer.getByRole('link', { name: 'Open article' }).press('Enter');
  await expect(page).toHaveURL(new RegExp(`/products/${own.id}$`));
  await expect(page.getByRole('main')).toBeFocused();
  await expect(page.getByRole('heading', { name: own.name })).toBeVisible();

  await page.getByRole('navigation', { name: 'Master data' }).getByRole('link', { name: 'Products', exact: true }).click();
  const main = page.getByRole('main');
  await main.getByLabel('SKU', { exact: true }).fill(own.sku);
  await main.getByRole('button', { name: 'Search', exact: true }).press('Enter');
  await expect(page).toHaveURL(new RegExp(`/products/${own.id}$`));
  await expect(page.getByRole('heading', { name: own.name })).toBeVisible();

  await page.locator('.user-preferences > summary').click();
  await page.getByLabel('Language', { exact: true }).selectOption('de');
  await page.keyboard.press('Meta+k');
  const deQuery = page.getByRole('searchbox', { name: 'Globale Suche' });
  await expect(deQuery).toBeFocused();
  await expect(deQuery).toHaveAttribute('placeholder', 'Frage stellen, suchen oder eine Aktion ausführen …');
  await deQuery.fill(`Zeige mir den Artikel mit SKU ${own.sku}`);
  await deQuery.press('Enter');
  const deAnswer = page.getByRole('region', { name: 'Suchergebnisse' });
  await expect(deAnswer.getByRole('status')).toHaveText('Ein passender Artikel');
  await expect(deAnswer.getByRole('heading', { name: own.name })).toBeVisible();
  for (const foreign of foreignStock) {
    await deQuery.fill(foreign.sku);
    await deQuery.press('Enter');
    await expect(deAnswer.getByRole('status')).toHaveText('Im ausgewählten Kontext entspricht kein Artikel diesem Filter.');
    await expect(deAnswer.getByRole('link')).toHaveCount(0);
  }
  await deQuery.fill(own.id);
  await deQuery.press('Enter');
  await expect(deAnswer.getByRole('status')).toHaveText('Ein passender Artikel');
  await expect(deAnswer).toContainText('Über Artikel-ID finden');
  await deQuery.press('Escape');
  await expect(deAnswer).toHaveCount(0);
  await expect(deQuery).toBeEmpty();
  expect(requests.every(method => method === 'HEAD' || method === 'GET')).toBe(true);
});

test('unauthorized discovery and results stay hidden; changing credential clears the input', async ({ page }) => {
  await page.goto('/local-access');
  await page.getByLabel('Demo token').fill(demoToken('synthetic-registration'));
  const discovery = page.waitForResponse(response =>
    response.url().endsWith('/v1/articles:find-by-sku') && response.request().method() === 'HEAD');
  // Dispatch both events in one browser task, before the credential effect can render.
  await page.getByRole('main').locator('form').evaluate((form: HTMLFormElement) => {
    form.requestSubmit();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true, cancelable: true }));
  });
  expect((await discovery).status()).toBe(403);
  await expect(page.getByRole('button', { name: 'Find article by SKU' })).toHaveCount(0);
  const query = page.getByRole('searchbox', { name: 'Global search' });
  await query.fill('PRIVATE-ARTICLE');
  await query.press('Enter');
  const results = page.getByRole('region', { name: 'Search results' });
  await expect(results.getByRole('status')).toHaveText(/Access denied/);
  await expect(results.getByRole('link')).toHaveCount(0);
  await page.getByRole('button', { name: 'Clear session credential' }).click();
  await expect(query).toBeEmpty();
  await expect(results).toHaveCount(0);
});

test('clarifies known queries, refuses unsupported writes and distinguishes errors from no matches', async ({ page }) => {
  await page.goto('/');
  await setDemoAccess(page, 'synthetic-fixture');
  const methods: string[] = [];
  await page.route('**/v1/**', async route => {
    const method = route.request().method();
    methods.push(method);
    if (method === 'HEAD') return route.fulfill({ status: 204 });
    const { searchParams } = new URL(route.request().url());
    switch (searchParams.get('sku')) {
      case 'MISSING': return route.fulfill({ status: 404 });
      case 'OFFLINE': return route.fulfill({ status: 503 });
      default: return route.fulfill({ json: { id: randomUUID(), sku: 'WRONG-LOOKUP', name: 'Do not render', revision: 1 } });
    }
  });
  const query = page.getByRole('searchbox', { name: 'Global search' });
  const status = page.getByRole('region', { name: 'Search results' }).getByRole('status');
  const ask = async (text: string) => { await query.fill(text); await query.press('Enter'); };
  await ask('Find article with SKU');
  await expect(status).toHaveText('Enter one article ID or one exact SKU.');
  await ask('Find article with SKU X or Y');
  await expect(status).toHaveText('Enter one article ID or one exact SKU.');
  await ask('Create a sales order');
  await expect(status).toHaveText(/cannot change records/);
  expect(methods).toContain('HEAD');
  expect(methods.filter(method => method !== 'HEAD')).toEqual([]);
  await ask('MISSING');
  await expect(status).toHaveText(/No article matches/);
  await ask('OFFLINE');
  await expect(status).toHaveText(/Check the local API/);
  await ask('UNEXPECTED');
  await expect(status).toHaveText(/unexpected response/);
  await expect(page.getByText('Do not render')).toHaveCount(0);
});

test('a late read does not replace a newer answer or restore cleared context', async ({ page }) => {
  await page.goto('/');
  await setDemoAccess(page, 'synthetic-fixture');
  let release!: () => void;
  let markSeen!: () => void;
  let markFinished!: () => void;
  const seen = new Promise<void>(resolve => { markSeen = resolve; });
  const pending = new Promise<void>(resolve => { release = resolve; });
  const finished = new Promise<void>(resolve => { markFinished = resolve; });
  await page.route('**/v1/articles:find-by-sku*', async route => {
    if (route.request().method() === 'HEAD') return route.fulfill({ status: 204 });
    const sku = new URL(route.request().url()).searchParams.get('sku');
    if (sku === 'OLD') {
      markSeen();
      await pending;
      try { await route.fulfill({ json: { id: randomUUID(), sku, name: 'Stale article', revision: 1 } }); }
      catch { /* An aborted browser request may no longer accept the delayed response. */ }
      finally { markFinished(); }
    } else {
      await route.fulfill({ json: { id: randomUUID(), sku, name: 'Current article', revision: 1 } });
    }
  });
  const query = page.getByRole('searchbox', { name: 'Global search' });
  const results = page.getByRole('region', { name: 'Search results' });
  await query.fill('OLD');
  await query.press('Enter');
  await seen;
  await expect(results.getByRole('status')).toHaveText(/Request in progress/);
  await query.fill('NEW');
  await query.press('Enter');
  await expect(results.getByRole('heading', { name: 'Current article' })).toBeVisible();
  release();
  await finished;
  await expect(results.getByRole('heading', { name: 'Current article' })).toBeVisible();
  await expect(results.getByText('Stale article')).toHaveCount(0);
  await page.getByRole('button', { name: 'Clear session credential' }).click();
  await expect(query).toBeEmpty();
  await expect(results).toHaveCount(0);
});
