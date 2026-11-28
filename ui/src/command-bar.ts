import { Component, computed, DestroyRef, effect, ElementRef, inject, signal, viewChild } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowUpRight, LucideSearch } from '@lucide/angular';
import { ArticleApi } from './article-api';
import type { ArticleLookupKind } from './article-identifiers';
import { commandAvailable, commandExample, discoveryLabels, maxCommandLength, normalizeCommand,
  queryOptions, resolveCommand, type CommandResult } from './command-intent';
import type { Message } from './messages';
import { Preferences } from './preferences';

@Component({
  selector: 'erp-command-bar',
  imports: [ReactiveFormsModule, RouterLink, LucideArrowUpRight, LucideSearch],
  template: `
    <form class="command-form" role="search" [attr.aria-label]="t()('commandSearch')"
      (focusin)="open()" (focusout)="leave($event)" (keydown.escape)="dismiss($event)"
      (submit)="$event.preventDefault(); execute()">
      <label class="sr-only" for="command-query">{{ t()('commandSearch') }}</label>
      <input #input id="command-query" type="search" [formControl]="query" [maxlength]="maxLength"
        [placeholder]="t()('commandPlaceholder')" [attr.aria-keyshortcuts]="keybinding"
        autocomplete="off" spellcheck="false">
      <button type="submit" class="command-submit"><svg lucideSearch aria-hidden="true"></svg>{{ t()('search') }}</button>
      <kbd aria-hidden="true">{{ shortcut }}</kbd>
      @if (opened() && (suggestions().length || notice() || result())) {
        <section class="command-panel" [attr.aria-label]="t()('commandResults')">
          @if (suggestions().length) {
            <p class="eyebrow">{{ t()('businessQueries') }}</p>
            <div class="actions">
              @for (kind of suggestions(); track kind) {
                <button type="button" (click)="useSuggestion(kind)">
                  <svg lucideSearch aria-hidden="true"></svg>{{ t()(labels[kind]) }}
                </button>
              }
            </div>
          }
          @let answer = result();
          <p role="status" class="command-status">
            @if (notice(); as key) { {{ t()(key) }} }
            @else if (answer) {
              {{ t()(answer.status === 'success' ? 'commandMatch' :
                answer.status === 'clarificationRequired' ? 'commandChooseArticle' : answer.message) }}
            }
          </p>
          @if (answer && answer.status === 'success') {
            <h2>{{ answer.article.name }}</h2>
            <a class="button" [routerLink]="['/products', answer.article.id]">
              <svg lucideArrowUpRight aria-hidden="true"></svg>{{ t()('openArticle') }}
            </a>
          } @else if (answer?.status === 'clarificationRequired') {
            <ul class="command-matches">
              @for (article of answer.matches; track article.id) {
                <li>
                  <a class="button" [routerLink]="['/products', article.id]">
                    <svg lucideArrowUpRight aria-hidden="true"></svg>{{ t()('openArticle') }}
                    <span class="technical">{{ article.sku }}</span>
                  </a>
                </li>
              }
            </ul>
            @if (answer.hasMore) { <p>{{ t()('commandMoreMatches') }}</p> }
          }
          @if (answer && answer.status !== 'unauthorized') {
            <dl class="entity-details command-evidence">
              <dt>{{ t()('commandInterpretation') }}</dt>
              @if (answer.lookup.operationId === 'findArticleBySku') {
                <dd>{{ t()(labels.sku) }}</dd>
                <dt>{{ t()('sku') }}</dt><dd class="technical">{{ answer.lookup.filter.sku }}</dd>
              } @else if (answer.lookup.operationId === 'findArticlesByName') {
                <dd>{{ t()(labels.name) }}</dd>
                <dt>{{ t()('name') }}</dt><dd class="exact-filter">“{{ answer.lookup.filter.name }}”</dd>
              } @else {
                <dd>{{ t()('lookup') }}</dd>
                <dt>{{ t()('id') }}</dt><dd class="technical">{{ answer.lookup.filter.id }}</dd>
              }
              <dt>{{ t()('commandScope') }}</dt><dd>{{ t()('scope') }}</dd>
              @if (answer.status === 'success' || answer.status === 'clarificationRequired') {
                <dt>{{ t()('commandSource') }}</dt><dd>{{ t()('masterData') }}</dd>
              }
              @if (answer.status === 'success') {
                @if (answer.lookup.operationId === 'findArticlesByName') {
                  <dt>{{ t()('sku') }}</dt><dd class="technical">{{ answer.article.sku }}</dd>
                }
                <dt>{{ t()('revision') }}</dt><dd>{{ answer.article.revision }}</dd>
              }
            </dl>
          }
        </section>
      }
    </form>
  `
})
export class CommandBar {
  private readonly api = inject(ArticleApi);
  private readonly prefs = inject(Preferences);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly input = viewChild.required<ElementRef<HTMLInputElement>>('input');
  private readonly authorized = signal<Partial<Record<ArticleLookupKind, number>>>({});
  private readonly typed = signal('');
  private readonly snapshot = signal<{ version: number; answer: CommandResult } | null>(null);
  private interactionVersion = this.api.credentialVersion();
  private lookup?: AbortController;
  private discovery?: AbortController;

  readonly t = this.prefs.t;
  readonly maxLength = maxCommandLength;
  readonly labels = discoveryLabels;
  readonly keybinding = /Mac/.test(navigator.platform) ? 'Meta+K' : 'Control+K';
  readonly shortcut = this.keybinding === 'Meta+K' ? '⌘ K' : 'Ctrl+K';
  readonly query = new FormControl('', { nonNullable: true });
  readonly opened = signal(false);
  readonly notice = signal<Message | null>(null);
  readonly result = computed(() => {
    const snapshot = this.snapshot();
    return snapshot?.version === this.api.credentialVersion() ? snapshot.answer : null;
  });
  readonly suggestions = computed(() => {
    if (this.notice() || this.result()) return [];
    const version = this.api.credentialVersion();
    const allowed = this.authorized();
    const input = normalizeCommand(this.typed()).toLowerCase();
    return queryOptions.filter(kind => {
      const example = commandExample(kind, this.prefs.locale());
      return allowed[kind] === version && input !== example && example.startsWith(input);
    });
  });

  constructor() {
    const destroy = inject(DestroyRef);
    this.query.valueChanges.pipe(takeUntilDestroyed(destroy)).subscribe(value => {
      this.open();
      this.typed.set(value);
      this.cancelLookup();
      this.notice.set(null);
    });
    destroy.onDestroy(() => this.close());
    effect(() => this.resetForCredential(this.api.credentialVersion()));
  }

  focus() {
    this.input().nativeElement.focus();
    this.open();
  }

  open() {
    // DOM focus can arrive before the scheduled effect after a credential change.
    // Adopt the live version now so the effect cannot close this new interaction.
    this.resetForCredential(this.api.credentialVersion());
    if (this.opened() || !commandAvailable) return;
    this.opened.set(true);
    if (!this.api.hasCredential()) return;
    const controller = this.discovery = new AbortController();
    const version = this.api.credentialVersion();
    for (const kind of queryOptions) {
      void this.api.canDiscover(kind, controller.signal).then(allowed => {
        if (allowed && !controller.signal.aborted && version === this.api.credentialVersion()) {
          this.authorized.update(known => ({ ...known, [kind]: version }));
        }
      });
    }
  }

  leave(event: FocusEvent) {
    if (!(event.relatedTarget instanceof Node) || !this.host.nativeElement.contains(event.relatedTarget)) this.close();
  }

  dismiss(event: Event) {
    event.preventDefault();
    event.stopPropagation();
    this.input().nativeElement.focus();
    this.reset();
  }

  close() {
    this.opened.set(false);
    this.authorized.set({});
    this.notice.set(null);
    this.discovery?.abort();
    this.cancelLookup();
  }

  reset() {
    this.close();
    this.query.setValue('', { emitEvent: false });
    this.typed.set('');
  }

  useSuggestion(kind: ArticleLookupKind) {
    if (!this.suggestions().includes(kind)) return;
    this.input().nativeElement.focus();
    this.query.setValue(`${commandExample(kind, this.prefs.locale())} `);
  }

  async execute() {
    this.open();
    this.cancelLookup();
    const resolution = resolveCommand(this.query.value);
    if (resolution.kind !== 'lookup') {
      this.notice.set(resolution.kind === 'clarificationRequired' && resolution.parameter === 'name'
        ? 'commandNameRequired' : resolution.kind);
      return;
    }
    this.notice.set('busy');
    const controller = this.lookup = new AbortController();
    const version = this.api.credentialVersion();
    const { lookup } = resolution;
    const response = lookup.operationId === 'getArticle'
      ? await this.api.get(lookup.filter.id, controller.signal)
      : lookup.operationId === 'findArticleBySku'
        ? await this.api.findBySku(lookup.filter.sku, controller.signal)
        : await this.api.findByName(lookup.filter.name, controller.signal);
    if (controller.signal.aborted || version !== this.api.credentialVersion()) return;
    this.notice.set(null);
    const base = { schemaVersion: 1, lookup } as const;
    let answer: CommandResult;
    if ('error' in response) {
      if (response.error === 'denied' || response.error === 'tokenMissing') {
        this.discovery?.abort();
        this.authorized.set({});
        answer = { ...base, status: 'unauthorized', message: response.error };
      } else if (response.error === 'missing') {
        answer = { ...base, status: 'noMatches', message: 'commandNoMatches' };
      } else if (response.error === 'unavailable') {
        answer = { ...base, status: 'serviceUnavailable', message: response.error };
      } else {
        answer = { ...base, status: 'executionError', message: 'executionError' };
      }
    } else if ('matches' in response) {
      if (response.matches.length === 0) answer = { ...base, status: 'noMatches', message: 'commandNoMatches' };
      else if (response.matches.length > 1) answer = { ...base, status: 'clarificationRequired', ...response };
      else answer = { ...base, status: 'success', article: response.matches[0] };
    } else {
      const matches = lookup.operationId === 'getArticle'
        ? response.article.id.toLowerCase() === lookup.filter.id
        : lookup.operationId === 'findArticleBySku' && response.article.sku === lookup.filter.sku;
      answer = matches ? { ...base, status: 'success', article: response.article }
        : { ...base, status: 'executionError', message: 'executionError' };
    }
    this.snapshot.set({ version, answer });
  }

  private cancelLookup() {
    this.lookup?.abort();
    this.snapshot.set(null);
  }

  private resetForCredential(version: number) {
    if (this.interactionVersion === version) return;
    this.interactionVersion = version;
    this.reset();
  }
}
