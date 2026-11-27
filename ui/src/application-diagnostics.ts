import { Component, effect, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { ArticleApi } from './article-api';
import type { Message } from './messages';
import { Preferences } from './preferences';
import { Feedback, PageHeading } from './ui';

type Readiness = 'ready' | 'unready' | 'unreachable';
interface Report {
  observed_at_unix_ms: number;
  application_checks: {service: string; readiness: Readiness}[];
  jetstream: {
    stream: {messages: number; bytes: number} | null;
    consumer: {pending: number; ack_pending: number; redelivered: number} | null;
  };
}

@Component({
  imports: [RouterLink, Feedback, PageHeading],
  template: `
    <erp-page-heading [title]="t()('administration')" [description]="t()('healthIntro')" />
    @if (!api.canReadHealth()) {
      <p>{{ t()('platformGrantRequired') }}</p><a routerLink="/local-access">{{ t()('access') }}</a>
    } @else {
      <section class="card">
        <h2>{{ t()('applicationHealth') }}</h2>
        <erp-feedback [message]="message() ? t()(message()!) : ''" />
        @if (report(); as latest) {
          <p>{{ t()('healthObservedAt') }}: {{ observed(latest.observed_at_unix_ms) }}</p>
          <table>
            <thead><tr><th scope="col">{{ t()('healthService') }}</th><th scope="col">{{ t()('healthReadiness') }}</th></tr></thead>
            <tbody>
              @for (row of latest.application_checks; track row.service) {
                <tr><th scope="row">{{ row.service }}</th><td>{{ status(row.readiness) }}</td></tr>
              }
            </tbody>
          </table>
          <h3>{{ t()('jetstreamCounters') }}</h3>
          <h4>ERP4US_BUSINESS_EVENTS_V1</h4>
          @if (latest.jetstream.stream; as stream) {
            <dl><dt>{{ t()('jetstreamMessages') }}</dt><dd>{{ stream.messages }}</dd>
            <dt>{{ t()('jetstreamBytes') }}</dt><dd>{{ stream.bytes }}</dd></dl>
          } @else { <p>{{ t()('jetstreamNotObserved') }}</p> }
          <h4>article-receipts</h4>
          @if (latest.jetstream.consumer; as consumer) {
            <dl><dt>{{ t()('jetstreamPending') }}</dt><dd>{{ consumer.pending }}</dd>
            <dt>{{ t()('jetstreamAckPending') }}</dt><dd>{{ consumer.ack_pending }}</dd>
            <dt>{{ t()('jetstreamRedelivered') }}</dt><dd>{{ consumer.redelivered }}</dd></dl>
          } @else { <p>{{ t()('jetstreamNotObserved') }}</p> }
        }
      </section>
    }
  `,
  styles: [`table {width:100%; border-collapse:collapse} td,th {text-align:left; padding:.5rem}`],
})
export class ApplicationDiagnostics {
  readonly api = inject(ArticleApi);
  private readonly preferences = inject(Preferences);
  readonly t = this.preferences.t;
  readonly report = signal<Report | null>(null);
  readonly message = signal<Message | null>(null);

  constructor() {
    effect(onCleanup => {
      const allowed = this.api.canReadHealth();
      const { Authorization } = this.api.bearerHeaders();
      this.report.set(null);
      if (!allowed) return;
      this.message.set('busy');
      const control = new AbortController();
      const refresh = () => void this.refresh(Authorization, control.signal);
      refresh();
      const timer = window.setInterval(refresh, 6000);
      onCleanup(() => { window.clearInterval(timer); control.abort(); });
    });
  }

  observed(ms: number) {
    return new Intl.DateTimeFormat(this.preferences.locale(), { dateStyle: 'medium', timeStyle: 'medium' }).format(ms);
  }

  status(state: Readiness) {
    const translation: Record<Readiness, Message> = { ready: 'healthReady', unready: 'healthUnready', unreachable: 'healthUnreachable' };
    return this.t()(translation[state]);
  }

  private async refresh(Authorization: string, signal: AbortSignal) {
    try {
      const response = await fetch('/v1/admin/application-health', {
        headers: { Authorization }, signal: AbortSignal.any([signal, AbortSignal.timeout(5000)]),
        cache: 'no-store', redirect: 'error',
      });
      if (!response.ok) throw new Error('diagnostics unavailable');
      const report: Report = await response.json();
      if (!signal.aborted) { this.report.set(report); this.message.set(null); }
    } catch {
      if (!signal.aborted) { this.report.set(null); this.message.set('unavailable'); }
    }
  }
}
