import { Component, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowUpRight, LucideBoxes, LucideClipboardList, LucideStar } from '@lucide/angular';
import { ArticleApi } from './article-api';
import { Preferences } from './preferences';
import { Feedback, PageHeading } from './ui';

@Component({
  imports: [PageHeading, RouterLink, LucideArrowUpRight, LucideBoxes, LucideClipboardList, LucideStar],
  template: `
    <p class="eyebrow">{{ t()('workspaceLabel') }}</p>
    <erp-page-heading [title]="t()('workspace')" [description]="t()('workspaceIntro')" />
    <section class="feature">
      <div class="feature-icon"><svg lucideBoxes aria-hidden="true"></svg></div>
      <div>
        <p class="eyebrow">{{ t()('masterData') }}</p>
        <h2>{{ t()('products') }}</h2>
        <p>{{ t()('productIntro') }}</p>
      </div>
      <a class="button primary" routerLink="/products">
        {{ t()('openProducts') }}<svg lucideArrowUpRight aria-hidden="true"></svg>
      </a>
    </section>
    <div class="widget-grid">
      <section class="card widget">
        <svg lucideClipboardList aria-hidden="true"></svg>
        <h2>{{ t()('tasks') }}</h2><p>{{ t()('tasksUnavailable') }}</p>
      </section>
      <section class="card widget">
        <svg lucideStar aria-hidden="true"></svg>
        <h2>{{ t()('favorites') }}</h2><p>{{ t()('favoritesUnavailable') }}</p>
      </section>
    </div>
  `
})
export class Workspace {
  readonly t = inject(Preferences).t;
}

@Component({
  imports: [Feedback, PageHeading, ReactiveFormsModule, RouterLink],
  template: `
    <erp-page-heading [title]="t()('access')" [description]="t()('accessIntro')" />
    <section class="card form-width">
      <form (submit)="$event.preventDefault(); connect()">
        <label for="credential">{{ t()('token') }}</label>
        <input id="credential" type="password" [formControl]="credential" required
          autocomplete="off" aria-describedby="credential-hint">
        <small id="credential-hint">{{ t()('hint') }}</small>
        <div class="actions">
          <button type="submit" class="primary">{{ t()('useCredential') }}</button>
          <button type="button" [disabled]="!api.hasCredential()" (click)="clear()">{{ t()('clearCredential') }}</button>
        </div>
      </form>
      <erp-feedback [message]="notice() ? t()(notice()!) : ''" />
      <a routerLink="/products">{{ t()('openProducts') }}</a>
    </section>
  `
})
export class DemoAccess {
  readonly t = inject(Preferences).t;
  readonly api = inject(ArticleApi);
  readonly credential = new FormControl('', { nonNullable: true });
  readonly notice = signal<'credentialReady' | 'credentialCleared' | 'tokenMissing' | null>(null);

  connect() {
    if (!this.api.setCredential(this.credential.value)) {
      this.notice.set('tokenMissing');
      return;
    }
    this.credential.reset();
    this.notice.set('credentialReady');
  }

  clear() {
    this.credential.reset();
    this.api.clearCredential();
    this.notice.set('credentialCleared');
  }
}
