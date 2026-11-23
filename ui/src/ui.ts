import { Component, inject, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Preferences } from './preferences';
import type { Message } from './messages';

export interface Crumb { readonly label: Message; readonly route: string }

// The service owns filtering, ordering and page boundaries. Do not fetch a full collection to paginate it.
export interface ListQuery<Column extends string, Filters> {
  readonly limit: number;
  readonly cursor?: string;
  readonly order?: { readonly column: Column; readonly direction: 'asc' | 'desc' };
  readonly filters: Filters;
}
export interface ListPage<Row> { readonly rows: readonly Row[]; readonly nextCursor?: string }

@Component({
  selector: 'erp-page-heading',
  imports: [RouterLink],
  template: `
    @if (crumbs().length) {
      <nav class="crumbs" [attr.aria-label]="t()('breadcrumbs')">
        <ol>@for (item of crumbs(); track item.route) {
          <li><a [routerLink]="item.route">{{ t()(item.label) }}</a><span aria-hidden="true"> /</span></li>
        }</ol>
      </nav>
    }
    <div class="page-heading">
      <div>
        <h1>{{ title() }}</h1>
        @if (description()) { <p>{{ description() }}</p> }
      </div>
      <div class="actions"><ng-content /></div>
    </div>
  `
})
export class PageHeading {
  readonly title = input.required<string>();
  readonly description = input<string>();
  readonly crumbs = input<readonly Crumb[]>([]);
  readonly t = inject(Preferences).t;
}

@Component({
  selector: 'erp-section',
  template: `<section class="card"><h2>{{ title() }}</h2><ng-content /></section>`
})
export class Section {
  readonly title = input.required<string>();
}

@Component({
  selector: 'erp-feedback',
  template: `<div role="status" class="feedback" [class]="tone()">@if (message()) { {{ message() }} }</div>`
})
export class Feedback {
  readonly message = input('');
  readonly tone = input<'neutral' | 'success' | 'error'>('neutral');
}
