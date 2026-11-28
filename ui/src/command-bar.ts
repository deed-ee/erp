import { Component, computed, DestroyRef, effect, ElementRef, inject, signal, viewChild } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { FormControl, ReactiveFormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowUpRight, LucideSearch } from '@lucide/angular';
import { ArticleApi } from './article-api';
import { commandAvailable, commandExample, discoveryLabel, maxCommandLength, normalizeCommand,
  resolveCommand, type CommandResult } from './command-intent';
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
      @if (opened() && (suggestion() || notice() || result())) {
        <section class="command-panel" [attr.aria-label]="t()('commandResults')">
          @if (suggestion()) {
            <p class="eyebrow">{{ t()('businessQueries') }}</p>
            <button type="button" (click)="useSuggestion()">
              <svg lucideSearch aria-hidden="true"></svg>{{ t()(label) }}
            </button>
          }
          @let answer = result();
          <p role="status" class="command-status">
            @if (notice(); as key) { {{ t()(key) }} }
            @else if (answer) { {{ t()(answer.status === 'success' ? 'commandMatch' : answer.message) }} }
          </p>
          @if (answer && answer.status === 'success') {
            <h2>{{ answer.article.name }}</h2>
            <a class="button" [routerLink]="['/products', answer.article.id]">
              <svg lucideArrowUpRight aria-hidden="true"></svg>{{ t()('openArticle') }}
            </a>
          }
          @if (answer && answer.status !== 'unauthorized') {
            <dl class="entity-details command-evidence">
              <dt>{{ t()('commandInterpretation') }}</dt>
              @if (answer.lookup.operationId === 'findArticleBySku') {
                <dd>{{ t()(label) }}</dd>
                <dt>{{ t()('sku') }}</dt><dd class="technical">{{ answer.lookup.filter.sku }}</dd>
              } @else {
                <dd>{{ t()('lookup') }}</dd>
                <dt>{{ t()('id') }}</dt><dd class="technical">{{ answer.lookup.filter.id }}</dd>
              }
              <dt>{{ t()('commandScope') }}</dt><dd>{{ t()('scope') }}</dd>
              @if (answer.status === 'success') {
                <dt>{{ t()('commandSource') }}</dt><dd>{{ t()('masterData') }}</dd>
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
  private readonly authorized = signal<number | null>(null);
  private readonly typed = signal('');
  private readonly snapshot = signal<{ version: number; answer: CommandResult } | null>(null);
  private interactionVersion = this.api.credentialVersion();
  private lookup?: AbortController;
  private discovery?: AbortController;

  readonly t = this.prefs.t;
  readonly maxLength = maxCommandLength;
  readonly label = discoveryLabel;
  readonly keybinding = /Mac/.test(navigator.platform) ? 'Meta+K' : 'Control+K';
  readonly shortcut = this.keybinding === 'Meta+K' ? '⌘ K' : 'Ctrl+K';
  readonly query = new FormControl('', { nonNullable: true });
  readonly opened = signal(false);
  readonly notice = signal<Message | null>(null);
  readonly result = computed(() => {
    const snapshot = this.snapshot();
    return snapshot?.version === this.api.credentialVersion() ? snapshot.answer : null;
  });
  readonly suggestion = computed(() => {
    if (this.authorized() !== this.api.credentialVersion() || this.notice() || this.result()) return false;
    const input = normalizeCommand(this.typed()).toLowerCase();
    const example = commandExample(this.prefs.locale());
    return input !== example && example.startsWith(input);
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
    void this.api.canFindBySku(controller.signal).then(allowed => {
      if (allowed && !controller.signal.aborted && version === this.api.credentialVersion()) {
        this.authorized.set(version);
      }
    });
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
    this.authorized.set(null);
    this.notice.set(null);
    this.discovery?.abort();
    this.cancelLookup();
  }

  reset() {
    this.close();
    this.query.setValue('', { emitEvent: false });
    this.typed.set('');
  }

  useSuggestion() {
    if (!this.suggestion()) return;
    this.input().nativeElement.focus();
    this.query.setValue(`${commandExample(this.prefs.locale())} `);
  }

  async execute() {
    this.open();
    this.cancelLookup();
    const resolution = resolveCommand(this.query.value);
    if (resolution.kind !== 'lookup') {
      this.notice.set(resolution.kind);
      return;
    }
    this.notice.set('busy');
    const controller = this.lookup = new AbortController();
    const version = this.api.credentialVersion();
    const { lookup } = resolution;
    const response = lookup.operationId === 'getArticle'
      ? await this.api.get(lookup.filter.id, controller.signal)
      : await this.api.findBySku(lookup.filter.sku, controller.signal);
    if (controller.signal.aborted || version !== this.api.credentialVersion()) return;
    this.notice.set(null);
    const base = { schemaVersion: 1, lookup } as const;
    let answer: CommandResult;
    if ('error' in response) {
      if (response.error === 'denied' || response.error === 'tokenMissing') {
        this.discovery?.abort();
        this.authorized.set(null);
        answer = { ...base, status: 'unauthorized', message: response.error };
      } else if (response.error === 'missing') {
        answer = { ...base, status: 'noMatches', message: 'commandNoMatches' };
      } else if (response.error === 'unavailable') {
        answer = { ...base, status: 'serviceUnavailable', message: response.error };
      } else {
        answer = { ...base, status: 'executionError', message: 'executionError' };
      }
    } else if (lookup.operationId === 'getArticle'
      ? response.article.id.toLowerCase() !== lookup.filter.id : response.article.sku !== lookup.filter.sku) {
      answer = { ...base, status: 'executionError', message: 'executionError' };
    } else {
      answer = { ...base, status: 'success', article: response.article };
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
