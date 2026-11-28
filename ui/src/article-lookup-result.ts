import { Component, computed, inject, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideArrowUpRight } from '@lucide/angular';
import type { ArticleLookupResult } from './article-lookup';
import type { Message } from './messages';
import { Preferences } from './preferences';

@Component({
  selector: 'erp-article-lookup-result',
  imports: [RouterLink, LucideArrowUpRight],
  template: `
    @let result = answer();
    <p role="status" class="lookup-result-status" [attr.id]="statusId()">
      @if (statusKey(); as key) { {{ t()(key) }} }
    </p>
    <ng-content select="[lookup-context]" />
    @if (result?.status === 'success') {
      @if (inSection()) {
        <h3 class="lookup-result-title">{{ result.article.name }}</h3>
      } @else {
        <h2 class="lookup-result-title">{{ result.article.name }}</h2>
      }
      <a class="button" [routerLink]="['/products', result.article.id]">
        <svg lucideArrowUpRight aria-hidden="true"></svg>{{ t()('openArticle') }}
      </a>
    } @else if (result?.status === 'clarificationRequired') {
      <ul class="lookup-candidates">
        @for (article of result.matches; track article.id) {
          <li>
            <a class="button" [routerLink]="['/products', article.id]">
              <svg lucideArrowUpRight aria-hidden="true"></svg>{{ t()('openArticle') }}
              <span class="technical">{{ article.sku }}</span>
            </a>
          </li>
        }
      </ul>
      @if (result.hasMore) { <p>{{ t()('commandMoreMatches') }}</p> }
    }
    @if (result && result.status !== 'unauthorized') {
      <dl class="entity-details lookup-evidence">
        <dt>{{ t()('commandInterpretation') }}</dt>
        @if (result.lookup.operationId === 'findArticleBySku') {
          <dd>{{ t()('masterData.article.findBySku') }}</dd>
          <dt>{{ t()('sku') }}</dt><dd class="technical">{{ result.lookup.filter.sku }}</dd>
        } @else if (result.lookup.operationId === 'findArticlesByName') {
          <dd>{{ t()('masterData.article.findByName') }}</dd>
          <dt>{{ t()('name') }}</dt><dd class="exact-filter">“{{ result.lookup.filter.name }}”</dd>
        } @else {
          <dd>{{ t()('lookup') }}</dd>
          <dt>{{ t()('id') }}</dt><dd class="technical">{{ result.lookup.filter.id }}</dd>
        }
        <dt>{{ t()('commandScope') }}</dt><dd>{{ t()('scope') }}</dd>
        @if (result.status === 'success' || result.status === 'clarificationRequired') {
          <dt>{{ t()('commandSource') }}</dt><dd>{{ t()('masterData') }}</dd>
        }
        @if (result.status === 'success') {
          @if (result.lookup.operationId === 'findArticlesByName') {
            <dt>{{ t()('sku') }}</dt><dd class="technical">{{ result.article.sku }}</dd>
          }
          <dt>{{ t()('revision') }}</dt><dd>{{ formatRevision(result.article.revision) }}</dd>
        }
      </dl>
    }
  `
})
export class ArticleLookupResultView {
  readonly answer = input<ArticleLookupResult | null>(null);
  readonly notice = input<Message | null>(null);
  readonly inSection = input(false);
  readonly statusId = input<string>();
  private readonly prefs = inject(Preferences);
  readonly t = this.prefs.t;
  readonly statusKey = computed<Message | null>(() => {
    const result = this.answer();
    return this.notice() ?? (result?.status === 'success' ? 'commandMatch'
      : result?.status === 'clarificationRequired' ? 'commandChooseArticle' : result?.message ?? null);
  });

  formatRevision(revision: number) { return new Intl.NumberFormat(this.prefs.locale()).format(revision); }
}
