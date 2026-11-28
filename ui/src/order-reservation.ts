import { Component, effect, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { LucideRefreshCw } from '@lucide/angular';
import { ArticleApi, isArticleId } from './article-api';
import type { Message } from './messages';
import { Preferences } from './preferences';
import { Feedback, PageHeading } from './ui';

interface Reservation {
  order_id: string; line_id: string; warehouse_id: string; sku: string; product_revision: string;
  unit: string; requested: string; reserved: string; outstanding: string; source_event_id: string;
}

@Component({
  imports: [RouterLink, LucideRefreshCw, Feedback, PageHeading],
  template: `
    <erp-page-heading [title]="t()('reservationLink')" [description]="t()('reservationIntro')" />
    <a routerLink="/sales/orders">{{ t()('salesOrders') }}</a>
    @if (!api.canReadReservation()) {
      <p>{{ t()('reservationDenied') }}</p>
    } @else {
      <section class="card">
        <erp-feedback [message]="message() ? t()(message()!) : ''" />
        @if (reservation(); as row) {
          <h2>{{ row.sku }} · {{ row.product_revision }}</h2>
          <dl>
            <dt>{{ t()('reservationRequested') }}</dt><dd>{{ row.requested }} {{ row.unit }}</dd>
            <dt>{{ t()('reservationReserved') }}</dt><dd>{{ row.reserved }} {{ row.unit }}</dd>
            <dt>{{ t()('reservationOutstanding') }}</dt><dd>{{ row.outstanding }} {{ row.unit }}</dd>
            <dt>{{ t()('reservationWarehouse') }}</dt><dd>{{ row.warehouse_id }}</dd>
            <dt>{{ t()('reservationEvent') }}</dt><dd>{{ row.source_event_id }}</dd>
          </dl>
        }
        <button type="button" (click)="refresh.update(n => n + 1)" [disabled]="message() === 'busy'">
          <svg lucideRefreshCw aria-hidden="true"></svg>{{ t()('reservationRefresh') }}
        </button>
      </section>
    }
  `,
})
export class OrderReservation {
  readonly api = inject(ArticleApi);
  readonly t = inject(Preferences).t;
  private readonly params = toSignal(inject(ActivatedRoute).paramMap);
  readonly refresh = signal(0);
  readonly message = signal<Message | null>(null);
  readonly reservation = signal<Reservation | null>(null);

  constructor() {
    effect(onCleanup => {
      this.refresh();
      const params = this.params();
      const allowed = this.api.canReadReservation();
      const headers = this.api.requestHeaders();
      this.reservation.set(null);
      this.message.set(null);
      if (!allowed) return;
      const order = params?.get('orderId');
      const line = params?.get('lineId');
      if (!order || !line || !isArticleId(order) || !isArticleId(line)) {
        this.message.set('reservationMissing');
        return;
      }
      const control = new AbortController();
      onCleanup(() => control.abort());
      this.message.set('busy');
      void this.load(order, line, headers, control.signal);
    });
  }

  private async load(order: string, line: string, headers: Record<string, string>, signal: AbortSignal) {
    try {
      for (let attempt = 0; attempt < 5 && !signal.aborted; attempt++) {
        if (attempt) await new Promise<void>(resolve => setTimeout(resolve, 750));
        if (signal.aborted) return;
        const response = await fetch(`/v1/inventory/reservations/${order}/${line}`, {
          headers, signal: AbortSignal.any([signal, AbortSignal.timeout(5000)]),
          cache: 'no-store', redirect: 'error',
        });
        if (response.status === 404) continue;
        const row: Reservation | null = response.ok ? await response.json() : null;
        if (!signal.aborted) {
          this.reservation.set(row);
          this.message.set(row ? null : response.status === 403 ? 'reservationDenied' : 'unavailable');
        }
        return;
      }
      if (!signal.aborted) this.message.set('reservationMissing');
    } catch {
      if (!signal.aborted) this.message.set('unavailable');
    }
  }
}
