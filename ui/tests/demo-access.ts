import { execFileSync } from 'node:child_process';
import { expect, type Page } from '@playwright/test';

export function demoToken(subject = 'synthetic-writer') {
  return execFileSync('python3', ['-c',
    'import sys; sys.path.insert(0, "tests/integration"); from identity_fixture import token; print(token(subject=sys.argv[1]))',
    subject], { cwd: '..', encoding: 'utf8' }).trim();
}

export async function setDemoAccess(page: Page, token: string) {
  const menu = page.locator('.user-preferences');
  await menu.locator('summary').click();
  await menu.getByRole('link').click();
  await page.getByLabel('Demo token').fill(token);
  await page.getByRole('button', { name: 'Use for this session' }).press('Enter');
  await expect(page.getByLabel('Demo token')).toBeEmpty();
}
