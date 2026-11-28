import { Component, effect, inject, input, signal } from '@angular/core';
import { LucideRefreshCw, LucideSend } from '@lucide/angular';
import { ArticleApi, isArticleId } from './article-api';
import type { Message } from './messages';
import { Preferences } from './preferences';
import { ProcurementTermsPanel, type PurchaseDraft } from './procurement-terms';
import { Feedback } from './ui';

interface Handoff { plan_id: string; event_id: string; state: 'pending' | 'published' }
interface Access {
  handoffPath: string; proposalPath: string; plan: string; headers: Record<string, string>;
  signal: AbortSignal; canReadProposal: boolean;
}

@Component({
  selector: 'erp-procurement-handoff',
  imports: [Feedback, LucideRefreshCw, LucideSend, ProcurementTermsPanel],
  template: `
    <section class="procurement">
      <h3>{{ t()('procurementDraft') }}</h3>
      <p>{{ t()('procurementDraftIntro') }}</p>
      <erp-feedback [message]="message() ? t()(message()!) : ''" />
      @if (handoff(); as sent) {
        <p>{{ t()(sent.state === 'published' ? 'procurementBrokerAck' : 'procurementQueued') }}</p>
        @if (draft(); as proposal) {
          <p>{{ t()('procurementDraftId') }}: {{ proposal.proposal_id }}</p>
          <table [attr.aria-label]="t()('procurementDraft')">
            <thead><tr><th scope="col">{{ t()('sku') }}</th><th scope="col">{{ t()('materialToBuy') }}</th></tr></thead>
            <tbody>
              @for (line of proposal.lines; track line.sku + '/' + (line.revision ?? '')) {
                <tr><th scope="row">{{ line.sku }} @if (line.revision) { · {{ line.revision }} }</th>
                  <td>{{ line.pieces }} {{ line.unit }}</td></tr>
              }
            </tbody>
          </table>
          <erp-procurement-terms [draft]="proposal" />
        } @else if (!api.canReadProposal()) {
          <p>{{ t()('procurementReadDenied') }}</p>
        }
        @if (!draft()) {
          <button type="button" (click)="refresh.update(n => n + 1)" [disabled]="busy()">
            <svg lucideRefreshCw aria-hidden="true"></svg>{{ t()('procurementRefresh') }}
          </button>
        }
      } @else if (hasShortages() && api.canSendMaterialPlan()) {
        <button type="button" (click)="send()" [disabled]="busy()">
          <svg lucideSend aria-hidden="true"></svg>{{ t()('procurementSend') }}
        </button>
      } @else if (!hasShortages()) {
        <p>{{ t()('procurementNoShortages') }}</p>
      }
    </section>
  `,
  styles: [`.procurement table {width:100%; border-collapse:collapse;}
    .procurement th,.procurement td {padding:.4rem; text-align:left;}`],
})
export class ProcurementHandoffPanel {
  readonly api = inject(ArticleApi);
  readonly t = inject(Preferences).t;
  readonly orderId = input.required<string>();
  readonly lineId = input.required<string>();
  readonly planId = input.required<string>();
  readonly hasShortages = input.required<boolean>();
  readonly busy = signal(false);
  readonly handoff = signal<Handoff | null>(null);
  readonly draft = signal<PurchaseDraft | null>(null);
  readonly message = signal<Message | null>(null);
  readonly refresh = signal(0);
  private access?: Access;

  constructor() {
    effect(onCleanup => {
      this.refresh();
      const ids = [this.orderId(), this.lineId(), this.planId()];
      const headers = this.api.requestHeaders();
      const canReadProposal = this.api.canReadProposal();
      const canReadPlan = this.api.canReadMaterialPlan();
      this.access = undefined;
      this.handoff.set(null);
      this.draft.set(null);
      this.message.set(null);
      this.busy.set(false);
      if (!canReadPlan || !ids.every(isArticleId)) return;
      const control = new AbortController();
      onCleanup(() => control.abort());
      const [order, line, plan] = ids;
      this.access = {
        handoffPath: `/v1/inventory/reservations/${order}/${line}/material-plan/procurement-handoff`,
        proposalPath: `/v1/procurement/proposals/by-plan/${plan}`,
        plan, headers, signal: control.signal, canReadProposal,
      };
      void this.update(this.access, false);
    });
  }

  send() {
    if (this.access && !this.busy() && !this.handoff() && this.hasShortages() && this.api.canSendMaterialPlan()) {
      void this.update(this.access, true);
    }
  }

  private async update(access: Access, send: boolean) {
    const {handoffPath, proposalPath, plan, headers, signal} = access;
    const request = (path: string, method = 'GET') => fetch(path, {
      method, headers, signal: AbortSignal.any([signal, AbortSignal.timeout(5000)]),
      cache: 'no-store', redirect: 'error',
    });
    this.busy.set(true);
    this.message.set('busy');
    try {
      const response = await request(handoffPath, send ? 'POST' : 'GET');
      if (response.status === 404 && !send) {
        if (!signal.aborted) this.message.set(null);
        return;
      }
      if (!response.ok) {
        if (!signal.aborted) this.message.set(response.status === 409 ? 'procurementHandoffRejected' : 'unavailable');
        return;
      }
      let sent: Handoff = await response.json();
      if (signal.aborted) return;
      if (sent.plan_id !== plan) throw new Error('Invalid handoff reference');
      this.handoff.set(sent);
      for (let attempt = 0; attempt < 6 && !signal.aborted; attempt++) {
        if (attempt) await new Promise<void>(resolve => setTimeout(resolve, 700));
        if (signal.aborted) return;
        if (sent.state === 'pending') {
          const progress = await request(handoffPath);
          if (!progress.ok) throw new Error('Handoff status unavailable');
          sent = await progress.json();
          if (signal.aborted) return;
          if (sent.plan_id !== plan) throw new Error('Invalid handoff reference');
          this.handoff.set(sent);
        }
        if (access.canReadProposal) {
          const reply = await request(proposalPath);
          if (reply.ok) {
            const proposal: PurchaseDraft = await reply.json();
            if (signal.aborted) return;
            if (proposal.plan_id !== plan || proposal.status !== 'draft') throw new Error('Invalid proposal reference');
            this.draft.set(proposal);
            this.message.set(null);
            return;
          }
          if (reply.status !== 404) throw new Error('Proposal unavailable');
        } else if (sent.state === 'published') break;
      }
      if (!signal.aborted) this.message.set(access.canReadProposal ? 'procurementDraftPending' : null);
    } catch {
      if (!signal.aborted) this.message.set('unavailable');
    } finally {
      if (!signal.aborted) this.busy.set(false);
    }
  }
}
