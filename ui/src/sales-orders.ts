import { Component, effect, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowRight, LucidePlus, LucideSave } from '@lucide/angular';
import { ArticleApi } from './article-api';
import type { Message } from './messages';
import { Preferences } from './preferences';
import { Feedback, PageHeading } from './ui';

interface Customer { id: string; name: string; code: string; profile?: {currency: string} }
interface Product { sku: string; revision: string; name: string; price_eur: string }
interface Draft {
  id: string; customer: Customer; status: 'draft'; currency: string; total: string;
  line: {sku: string; product_revision: string; pieces: string; unit_price: string; unit: string};
}
interface Page<T> { items: T[]; next_after: string | null }

async function fetchJson<T>(path: string, headers: Record<string, string>, signal: AbortSignal, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    headers: body === undefined ? headers : { ...headers, 'Content-Type': 'application/json' },
    method: body === undefined ? 'GET' : 'POST',
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.any([signal, AbortSignal.timeout(8000)]),
    cache: 'no-store', redirect: 'error',
  });
  if (!response.ok) throw response.status;
  return response.json();
}

@Component({
  imports: [FormsModule, RouterLink, LucideArrowRight, LucidePlus, LucideSave, Feedback, PageHeading],
  template: `
    <erp-page-heading [title]="t()('salesOrders')" [description]="t()('salesIntro')" />
    @if (!api.canReadSales()) {
      <p>{{ t()('salesReadRequired') }}</p><a routerLink="/local-access">{{ t()('access') }}</a>
    } @else {
      @if (api.canCreateSalesDraft()) {
        <form class="card form-width" (ngSubmit)="create()">
          <h2>{{ t()('salesNewDraft') }}</h2>
          <label for="sales-customer">{{ t()('customer') }}</label>
          <select id="sales-customer" name="customer" [(ngModel)]="customerIndex" [disabled]="saving() || !!saved()">
            @for (item of customers(); track item.id; let i = $index) {
              <option [ngValue]="i">{{ item.code }} · {{ item.name }}</option>
            }
          </select>
          @if (nextCustomer(); as cursor) {
            <button type="button" (click)="moreCustomers(cursor)" [disabled]="saving() || !!saved()">
              <svg lucideArrowRight aria-hidden="true"></svg>{{ t()('salesNextCustomers') }}
            </button>
          }
          <label for="sales-product">{{ t()('planningProduct') }}</label>
          <select id="sales-product" name="product" [(ngModel)]="productIndex" [disabled]="saving() || !!saved()">
            @for (item of products(); track $index; let i = $index) {
              <option [ngValue]="i">{{ item.sku }} · {{ item.name }} ({{ item.revision }}) · EUR {{ item.price_eur }}</option>
            }
          </select>
          <label for="sales-pieces">{{ t()('planningPieces') }}</label>
          <input id="sales-pieces" name="pieces" inputmode="numeric" autocomplete="off" [(ngModel)]="pieces"
            [disabled]="saving() || !!saved()" required>
          <div class="actions">
            @if (saved(); as order) {
              <p>{{ t()('salesSaved') }}: {{ order.currency }} {{ order.total }}</p>
              <button type="button" (click)="newDraft()">
                <svg lucidePlus aria-hidden="true"></svg>{{ t()('salesNewDraft') }}
              </button>
            } @else {
              <button type="submit" class="primary" [disabled]="saving() || !customers().length || !products().length">
                <svg lucideSave aria-hidden="true"></svg>{{ t()('salesSaveDraft') }}
              </button>
            }
          </div>
          <erp-feedback [message]="formMessage() ? t()(formMessage()!) : ''" />
        </form>
      }
      <section class="card sales-list">
        <h2>{{ t()('salesDrafts') }}</h2>
        <erp-feedback [message]="listMessage() ? t()(listMessage()!) : ''" />
        @if (orders().items.length) {
          <table>
            <thead><tr><th scope="col">{{ t()('customer') }}</th><th scope="col">{{ t()('planningProduct') }}</th>
            <th scope="col">{{ t()('planningPieces') }}</th><th scope="col">{{ t()('salesOrderTotal') }}</th></tr></thead>
            <tbody>
              @for (order of orders().items; track order.id) {
                <tr><th scope="row">{{ order.customer.name }}</th><td>{{ order.line.sku }} ({{ order.line.product_revision }})</td>
                <td>{{ order.line.pieces }} {{ order.line.unit }}</td><td>{{ order.currency }} {{ order.total }}</td></tr>
              }
            </tbody>
          </table>
        }
        @if (orders().next_after; as cursor) {
          <button type="button" (click)="moreOrders(cursor)">
            <svg lucideArrowRight aria-hidden="true"></svg>{{ t()('nextPage') }}
          </button>
        }
      </section>
    }
  `,
  styles: [`.sales-list { overflow:auto; } table { width:100%; border-collapse:collapse; }
    td, th { text-align:left; padding:.5rem; }`],
})
export class SalesOrders {
  readonly api = inject(ArticleApi);
  readonly t = inject(Preferences).t;
  readonly customers = signal<Customer[]>([]);
  readonly products = signal<Product[]>([]);
  readonly nextCustomer = signal<string | null>(null);
  readonly orders = signal<Page<Draft>>({items: [], next_after: null});
  readonly formMessage = signal<Message | null>(null);
  readonly listMessage = signal<Message | null>(null);
  readonly saving = signal(false);
  readonly saved = signal<Draft | null>(null);
  customerIndex = 0;
  productIndex = 0;
  pieces = '';
  private access?: {headers: Record<string, string>; signal: AbortSignal};
  private orderLoad?: AbortController;
  private customerLoad?: AbortController;
  private command?: {identity: string; id: string};

  constructor() {
    effect(onCleanup => {
      const canRead = this.api.canReadSales();
      const canCreate = this.api.canCreateSalesDraft();
      const headers = this.api.requestHeaders();
      this.access = undefined;
      this.customers.set([]);
      this.products.set([]);
      this.nextCustomer.set(null);
      this.orders.set({items: [], next_after: null});
      this.newDraft();
      this.productIndex = 0;
      this.customerIndex = 0;
      this.saving.set(false);
      if (!canRead) return;
      const active = new AbortController();
      this.access = {headers, signal: active.signal};
      onCleanup(() => active.abort());
      void this.moreOrders();
      if (canCreate) {
        void this.moreCustomers();
        void this.loadProducts(this.access);
      }
    });
  }

  async moreOrders(cursor?: string) {
    const access = this.access;
    if (!access) return;
    this.orderLoad?.abort();
    this.orderLoad = new AbortController();
    const signal = AbortSignal.any([access.signal, this.orderLoad.signal]);
    this.listMessage.set('busy');
    try {
      const path = '/v1/sales/orders' + (cursor ? `?after=${encodeURIComponent(cursor)}` : '');
      const page = await fetchJson<Page<Draft>>(path, access.headers, signal);
      if (!signal.aborted) {
        this.orders.set(page);
        this.listMessage.set(page.items.length ? null : 'salesNoDrafts');
      }
    } catch { if (!signal.aborted) this.listMessage.set('unavailable'); }
  }

  async moreCustomers(cursor?: string) {
    const access = this.access;
    if (!access) return;
    this.customerLoad?.abort();
    this.customerLoad = new AbortController();
    const signal = AbortSignal.any([access.signal, this.customerLoad.signal]);
    try {
      const query = new URLSearchParams({role: 'customer', limit: '50'});
      if (cursor) query.set('after', cursor);
      const page = await fetchJson<Page<Customer>>(`/v1/partners?${query}`, access.headers, signal);
      if (!signal.aborted) {
        this.customerIndex = 0;
        this.customers.set(page.items.filter(customer => customer.profile?.currency === 'EUR'));
        this.nextCustomer.set(page.next_after);
      }
    } catch { if (!signal.aborted) this.formMessage.set('unavailable'); }
  }

  private async loadProducts({headers, signal}: NonNullable<SalesOrders['access']>) {
    try {
      const products = await fetchJson<Product[]>('/v1/manufacturing/products', headers, signal);
      if (!signal.aborted) this.products.set(products);
    } catch { if (!signal.aborted) this.formMessage.set('unavailable'); }
  }

  newDraft() {
    this.command = undefined;
    this.saved.set(null);
    this.formMessage.set(null);
    this.pieces = '';
  }

  async create() {
    const access = this.access;
    const customer = this.customers()[this.customerIndex];
    const product = this.products()[this.productIndex];
    if (!access || this.saving() || this.saved() || !this.api.canCreateSalesDraft()) return;
    if (!customer || !product || !/^(?:[1-9][0-9]{0,2}|1000)$/.test(this.pieces)) {
      this.formMessage.set('salesInvalid');
      return;
    }
    const input = {customer_id: customer.id, sku: product.sku, product_revision: product.revision, pieces: this.pieces};
    const identity = JSON.stringify(input);
    if (this.command?.identity !== identity) this.command = {identity, id: crypto.randomUUID()};
    this.saving.set(true);
    this.formMessage.set('busy');
    try {
      const draft = await fetchJson<Draft>('/v1/sales/orders', access.headers, access.signal, { ...input, id: this.command.id });
      if (!access.signal.aborted) {
        this.saved.set(draft);
        this.formMessage.set(null);
        void this.moreOrders();
      }
    } catch (status) {
      if (!access.signal.aborted) this.formMessage.set(status === 422 ? 'salesInvalid' : 'unavailable');
    } finally { if (!access.signal.aborted) this.saving.set(false); }
  }
}
