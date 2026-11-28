import { createHash, randomBytes } from 'node:crypto';
import { createServer } from 'node:http';
import { expect, test, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const origin = process.env.ERP4US_THEME_ORIGIN ?? '';
const password = process.env.ERP4US_THEME_PASSWORD ?? '';
const callbackPort = Number(process.env.ERP4US_THEME_CALLBACK_PORT);
const callback = `http://127.0.0.1:${callbackPort}/return`;
if (!/^https:\/\/127\.0\.0\.1:\d+$/.test(origin) || !/^Theme-[a-f0-9]{40}$/.test(password) ||
  !Number.isInteger(callbackPort) || callbackPort < 1024 || callbackPort > 65535) {
  throw new Error('Start the disposable loopback realm with tests/integration/keycloak-theme.sh.');
}

function authorize(locale: 'en' | 'de' = 'en') {
  const state = randomBytes(20).toString('hex');
  const challenge = createHash('sha256').update(randomBytes(32).toString('base64url')).digest('base64url');
  const parameters = new URLSearchParams({
    client_id: 'theme-browser', response_type: 'code', redirect_uri: callback, scope: 'openid',
    state, code_challenge: challenge, code_challenge_method: 'S256', ui_locales: locale,
  });
  return { url: `${origin}/realms/theme-check/protocol/openid-connect/auth?${parameters}`, state };
}

async function hasTheme(page: Page) {
  await expect(page.locator('#kc-header-wrapper')).toHaveText('erp4us');
  await expect(page.locator('body')).toHaveCSS('--erp-primary', await page.locator('body').evaluate(el =>
    el.ownerDocument.documentElement.classList.contains('pf-v5-theme-dark') ? '#65b8c1' : '#216f76'));
}

async function scan(page: Page) {
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']).analyze();
  expect(results.violations).toEqual([]);
}

test('real Keycloak desktop theme leaves rejected and successful OIDC flows intact', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.emulateMedia({ colorScheme: 'light' });
  const { url, state } = authorize();
  const external = new Set<string>();
  page.on('request', request => {
    const target = new URL(request.url());
    if (target.origin !== origin && target.origin !== new URL(callback).origin) external.add(target.host);
  });
  // Use a real, exact loopback destination; do not rely on intercepting a cross-origin redirect.
  const receiver = createServer((request, response) => {
    if (request.method !== 'GET' || request.url?.split('?')[0] !== '/return') {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, {
      'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store', 'Referrer-Policy': 'no-referrer'
    }).end('<!DOCTYPE html><html lang="en"><title>Theme check</title><h1>Synthetic return destination</h1></html>');
  });
  await new Promise<void>((resolve, reject) => {
    receiver.once('error', reject);
    receiver.listen(callbackPort, '127.0.0.1', resolve);
  });
  try {
    await page.goto(url);
    await hasTheme(page);
    const hasDecor = await page.locator('.pf-v5-c-login').evaluate(el => {
      const style = getComputedStyle(el, '::after');
      return style.backgroundImage.includes('business-network.svg') && Number.parseFloat(style.height) >= 400;
    });
    expect(hasDecor).toBe(true);
    await scan(page);
    await page.screenshot({ path: testInfo.outputPath('sign-in-desktop.png') }); // No credentials entered.

    await page.locator('#username').fill('theme-reader');
    await page.locator('#password').fill('invalid-test-password');
    await page.locator('#password').press('Enter');
    await expect(page.getByText('Invalid username or password.', { exact: true })).toBeVisible();
    await hasTheme(page);
    await page.locator('#username').fill('theme-reader');
    await page.locator('#password').fill(password);
    await page.locator('#password').press('Enter');
    await expect(page.getByRole('heading', { name: 'Synthetic return destination' })).toBeVisible();
    const result = new URL(page.url());
    expect(result.origin + result.pathname).toBe(callback);
    expect(result.searchParams.get('state')).toBe(state);
    expect(Boolean(result.searchParams.get('code'))).toBe(true);
    expect([...external]).toEqual([]);
  } finally {
    receiver.close();
    receiver.closeAllConnections();
  }
});

test('mobile German login uses the provider locale and recovery flow', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 320, height: 740 });
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto(authorize().url);
  await hasTheme(page);
  await page.locator('#login-select-toggle').selectOption({ label: 'German (Deutsch)' });
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  await hasTheme(page);
  const decoration = await page.locator('.pf-v5-c-login').evaluate(el =>
    getComputedStyle(el, '::after').content);
  expect(decoration).toBe('none');
  expect(await page.locator('html').evaluate(el => el.scrollWidth)).toBe(320);
  await page.locator('#username').focus();
  await page.keyboard.press('Tab');
  await expect(page.locator('#password')).toBeFocused();
  await scan(page);
  await page.mouse.click(8, 8);
  await page.screenshot({ path: testInfo.outputPath('sign-in-mobile.png') });

  await page.getByRole('link', { name: /Passwort vergessen/i }).press('Enter');
  await expect(page.locator('#kc-page-title')).toHaveText(/Passwort/i);
  await expect(page.locator('#kc-reset-password-form')).toBeVisible();
  await hasTheme(page);
  await page.getByRole('link', { name: /zurück/i }).press('Enter');
  await expect(page.locator('#password')).toBeVisible();
});

test('Keycloak required password update still renders its native controls', async ({ page }) => {
  await page.goto(authorize().url);
  await page.locator('#username').fill('theme-update');
  await page.locator('#password').fill(password);
  await page.locator('#password').press('Enter');
  await expect(page.locator('#password-new')).toBeVisible();
  await expect(page.locator('#password-confirm')).toBeVisible();
  await expect(page.locator('#kc-page-title')).toHaveText(/update password/i);
  await hasTheme(page);
});
