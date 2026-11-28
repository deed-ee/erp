import { expect, type Browser, type Page } from '@playwright/test';

// This probe uses the real browser login against the two HTTP owners. It does
// not claim that the normal browser receiving controls are wired.
type Batch = { plan_id: string; order_id: string; line_id: string };
type Login = (page: Page, user: 'purchase-receiver') => Promise<void>;

const tenant = 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5';
const organization = '3e793fb5-75a7-45f6-8f66-45a58fcec674';
const otherTenant = 'a260610d-ebcf-4bb7-b871-9d221ace30a8';
const otherOrganization = '2886c61e-85d8-4688-813a-b59c6641f9f8';
const id = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/;

async function withReceiver<T>(
  browser: Browser, login: Login,
  run: (page: Page, headers: Record<string, string>) => Promise<T>,
): Promise<T> {
  const context = await browser.newContext({ baseURL: 'http://127.0.0.1:4200', ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const session = page.waitForRequest(request => request.url().endsWith('/v1/session'));
    await login(page, 'purchase-receiver');
    const bearer = (await (await session).allHeaders())['authorization'];
    if (!bearer?.startsWith('Bearer ')) throw new Error('No verified receiver session request');
    return await run(page, {
      Authorization: bearer, 'X-Erp4us-Tenant': tenant, 'X-Erp4us-Organization': organization,
    });
  } finally {
    await context.close();
  }
}

export async function verifyReceiptOwners(browser: Browser, login: Login, batch: Batch) {
  for (const value of [batch.plan_id, batch.order_id, batch.line_id]) expect(value).toMatch(id);
  const sourcePath = `/v1/procurement/proposals/by-plan/${batch.plan_id}/purchase-orders`;
  const planPath = `/v1/inventory/reservations/${batch.order_id}/${batch.line_id}/material-plan`;
  const { receiptPath, receiptId, originalPlan } = await withReceiver(browser, login, async (page, headers) => {
    const source = await page.request.get(`${sourcePath}/receipt-source`, { headers });
    expect(source.status()).toBe(200);
    expect(source.headers()['cache-control']).toBe('no-store');
    const result = await source.json();
    expect(result.plan_id).toBe(batch.plan_id);
    expect(result.lines).toHaveLength(5);
    const plates = (result.lines as Array<Record<string, unknown>>).filter(line => line['sku'] === 'NM-ENDPLATE');
    expect(plates).toHaveLength(1);
    const line = plates[0];
    expect(line).toEqual({
      purchase_order_id: expect.stringMatching(id), purchase_line_id: expect.stringMatching(id),
      sku: 'NM-ENDPLATE', revision: null, unit: 'EA', ordered_pieces: '12',
    });
    expect((await page.request.get(sourcePath, { headers })).status()).toBe(403);
    const receiptPath = `${planPath}/purchase-orders/${line['purchase_order_id']}/lines/${line['purchase_line_id']}/receipts`;
    for (const denied of [
      { ...headers, 'X-Erp4us-Tenant': otherTenant },
      { ...headers, 'X-Erp4us-Organization': otherOrganization },
    ]) {
      expect((await page.request.get(`${sourcePath}/receipt-source`, { headers: denied })).status()).toBe(403);
      expect((await page.request.get(`${receiptPath}/intent`, { headers: denied })).status()).toBe(403);
    }
    const plan = await page.request.get(planPath, { headers });
    expect(plan.status()).toBe(200);
    const originalPlan = await plan.json();
    expect(originalPlan.materials).toContainEqual(expect.objectContaining({
      sku: 'NM-ENDPLATE', requested: '20', reserved: '8', uncovered: '12',
    }));
    expect((await page.request.get(receiptPath, { headers })).status()).toBe(404);
    const prepared = await page.request.post(`${receiptPath}/intent`, {
      headers, data: { pieces: '6', acknowledged_receipt_id: null },
    });
    expect(prepared.status()).toBe(201);
    const intent = await prepared.json();
    expect(intent).toEqual({ state: 'prepared', pieces: '6', receipt_id: expect.stringMatching(id) });
    expect((await page.request.get(receiptPath, { headers })).status()).toBe(404);

    let ownerStatus: number | undefined;
    // Let the request really commit, but drop its response before the browser
    // can see the receipt. The next session must recover by line, not by ID.
    await page.route(`**${receiptPath}`, async route => {
      try {
        const delivered = await route.fetch({ maxRedirects: 0, timeout: 15000 });
        ownerStatus = delivered.status();
      } finally {
        await route.abort('connectionreset');
      }
    }, { times: 1 });
    const clientSawResponse = await page.evaluate(async ({ path, headers, receipt }) => {
      try {
        await fetch(path, {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ receipt_id: receipt, pieces: '6' }),
        });
        return true;
      } catch {
        return false;
      }
    }, { path: receiptPath, headers, receipt: intent.receipt_id as string });
    expect(clientSawResponse).toBe(false);
    expect(ownerStatus).toBe(201);
    return { receiptPath, receiptId: intent.receipt_id as string, originalPlan };
  });

  await withReceiver(browser, login, async (page, headers) => {
    const reopened = await page.request.get(`${receiptPath}/intent`, { headers });
    expect(reopened.status()).toBe(200);
    const original = await reopened.json();
    expect(original).toEqual({
      state: 'confirmed', receipt: expect.objectContaining({
        receipt_id: receiptId,
        pieces: '6',
        allocation_after: expect.objectContaining({
          through_receipt_sequence: 1, receipt_allocated_pieces: '6',
          plan_held_pieces: '14', line_open_pieces: '6',
        }),
      }),
    });
    const replayed = await page.request.post(receiptPath, { headers, data: { receipt_id: receiptId, pieces: '6' } });
    expect(replayed.status()).toBe(200);
    expect(await replayed.json()).toEqual(original.receipt);
    const second = await page.request.post(`${receiptPath}/intent`, {
      headers, data: { pieces: '6', acknowledged_receipt_id: receiptId },
    });
    expect(second.status()).toBe(201);
    const later = await second.json();
    expect(later.receipt_id).toMatch(id);
    expect(later.receipt_id).not.toBe(receiptId);
    const posted = await page.request.post(receiptPath, {
      headers, data: { receipt_id: later.receipt_id, pieces: '6' },
    });
    expect(posted.status()).toBe(201);
    const final = await posted.json();
    expect(final.allocation_after).toMatchObject({
      through_receipt_sequence: 2, receipt_allocated_pieces: '12', plan_held_pieces: '20',
      line_open_pieces: '0', plan_remaining_shortage_pieces: '0',
    });
    const excessive = await page.request.post(`${receiptPath}/intent`, {
      headers, data: { pieces: '1', acknowledged_receipt_id: later.receipt_id },
    });
    expect(excessive.status()).toBe(409);
    expect(await excessive.json()).toEqual({ code: 'over_receipt' });
    const allocation = await page.request.get(receiptPath, { headers });
    expect(allocation.status()).toBe(200);
    expect(await allocation.json()).toEqual(final.allocation_after);
    const unchangedPlan = await page.request.get(planPath, { headers });
    expect(unchangedPlan.status()).toBe(200);
    expect(await unchangedPlan.json()).toEqual(originalPlan);
  });
}
