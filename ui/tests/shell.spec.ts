import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('keyboard navigation, focus, fixed demo context and persistent preferences', async ({ page }) => {
  await page.goto('/');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
  await expect(page.getByRole('banner')).toContainText('Fixed demo scope');
  await expect(page.getByLabel('Demo token')).toHaveCount(0);
  await page.getByRole('link', { name: 'Open products' }).press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
  const nav = page.getByRole('navigation', { name: 'Master data' });
  await nav.getByRole('button', { name: 'Catalog' }).click();
  await expect(nav.getByRole('link', { name: 'Products' })).toBeHidden();
  await page.getByRole('button', { name: 'Module navigation' }).click();
  await expect(nav).toBeHidden();
  const help = page.getByRole('button', { name: 'View help' });
  await help.press('Enter');
  const panelTitle = page.getByRole('heading', { name: 'Product guide' });
  await expect(panelTitle).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(help).toBeFocused();
  const rail = page.getByRole('navigation', { name: 'Modules' });
  const mainNavigation = rail.getByRole('button', { name: 'Menu' });
  await mainNavigation.press('Enter');
  await expect(mainNavigation).toHaveAttribute('aria-pressed', 'true');
  await expect(rail.getByRole('link', { name: 'Master data' })).toBeVisible();
  await page.locator('.user-preferences > summary').press('Enter');
  await page.getByLabel('Language', { exact: true }).selectOption('de');
  await page.getByLabel('Farbschema').selectOption('dark');
  await page.getByLabel('Anzeigedichte').selectOption('compact');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Produkte', exact: true })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('html')).toHaveAttribute('data-density', 'compact');
  await expect(page.getByRole('navigation', { name: 'Stammdaten' })).toBeHidden();
  await expect(page.getByRole('button', { name: 'Menü' })).toHaveAttribute('aria-pressed', 'true');
});

test('cancelled navigation preserves unsubmitted edits', async ({ page }) => {
  await page.goto('/products/new');
  await page.getByLabel('Name', { exact: true }).fill('Unsubmitted name');
  page.once('dialog', dialog => dialog.dismiss());
  await page.getByRole('link', { name: 'Cancel' }).click();
  await expect(page.getByLabel('Name', { exact: true })).toHaveValue('Unsubmitted name');
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('link', { name: 'Cancel' }).click();
  await expect(page).toHaveURL(/\/products$/);
});

const designs = [
  { name: 'workspace-light', path: '/', color: 'light', width: 1280, height: 800 },
  { name: 'products-dark', path: '/products', color: 'dark', width: 1280, height: 800 },
  { name: 'products-mobile', path: '/products', color: 'light', width: 390, height: 844 }
] as const;

for (const design of designs) {
  test(`accessibility and visual baseline: ${design.name}`, async ({ page }) => {
    await page.setViewportSize({ width: design.width, height: design.height });
    await page.emulateMedia({ colorScheme: design.color, reducedMotion: 'reduce' });
    await page.goto(design.path);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.locator('html')).toHaveCSS('color-scheme', design.color);
    const scan = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']).analyze();
    expect(scan.violations).toEqual([]);
    await expect(page).toHaveScreenshot(`${design.name}.png`, { fullPage: true, maxDiffPixelRatio: 0.002 });
    if (design.width < 400) {
      await page.setViewportSize({ width: 320, height: 720 });
      const nav = page.getByRole('navigation', { name: 'Master data' });
      const toggle = page.getByRole('button', { name: 'Module navigation' });
      await expect(nav).toBeHidden();
      await toggle.press('Enter');
      const catalog = nav.getByRole('button', { name: 'Catalog' });
      await expect(catalog).toBeFocused();
      await catalog.press('Escape');
      await expect(toggle).toBeFocused();
      expect(await page.locator('html').evaluate(el => el.scrollWidth)).toBe(320);
    }
  });
}
