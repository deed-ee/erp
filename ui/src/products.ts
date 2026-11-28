import { Component, DestroyRef, ElementRef, inject, signal, viewChild, HostListener } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { ActivatedRoute, type CanDeactivateFn, Router, RouterLink } from '@angular/router';
import { FormControl, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { LucidePackageSearch, LucidePlus, LucideSearch } from '@lucide/angular';
import { ArticleApi, type Article, isLoopback, type Result } from './article-api';
import { isArticleId, normalizeArticleSku } from './article-identifiers';
import { Preferences } from './preferences';
import { Feedback, PageHeading, Section, type Crumb } from './ui';
import type { Message } from './messages';

const workspaceCrumb = [{ label: 'workspace', route: '/' }] as const satisfies readonly Crumb[];
const productsCrumb = [...workspaceCrumb, { label: 'products', route: '/products' }] as const satisfies readonly Crumb[];

@Component({
  template: `<p>{{ t()('guideLookup') }}</p><p>{{ t()('guideRegister') }}</p>`
})
export class ProductGuide { readonly t = inject(Preferences).t; }

@Component({
  imports: [Feedback, PageHeading, Section, ReactiveFormsModule, RouterLink, LucidePackageSearch, LucidePlus, LucideSearch],
  template: `
    <erp-page-heading [title]="t()('products')" [description]="t()('productIntro')" [crumbs]="crumbs">
      <a class="button primary" routerLink="/products/new"><svg lucidePlus aria-hidden="true"></svg>{{ t()('register') }}</a>
    </erp-page-heading>
    <erp-section [title]="t()('lookup')">
      <form class="lookup-form" novalidate (submit)="$event.preventDefault(); lookup()">
        <div><label for="id">{{ t()('id') }}</label>
          <input #idInput id="id" [formControl]="id" required spellcheck="false"
            [attr.aria-invalid]="invalid()" aria-describedby="lookup-feedback">
        </div>
        <button type="submit"><svg lucideSearch aria-hidden="true"></svg>{{ t()('retrieve') }}</button>
      </form>
      <erp-feedback id="lookup-feedback" tone="error" [message]="invalid() ? t()('invalidId') : ''" />
    </erp-section>
    <erp-section class="lookup-section" [title]="t()('masterData.article.findBySku')">
      <form class="lookup-form" novalidate (submit)="$event.preventDefault(); lookupSku()" [attr.aria-busy]="skuStatus() === 'busy'">
        <div><label for="lookup-sku">{{ t()('sku') }}</label>
          <input #skuInput id="lookup-sku" [formControl]="sku" required maxlength="32"
            spellcheck="false" autocomplete="off" [attr.aria-invalid]="skuStatus() === 'invalidSku'"
            aria-describedby="sku-lookup-feedback">
        </div>
        <button type="submit"><svg lucideSearch aria-hidden="true"></svg>{{ t()('search') }}</button>
      </form>
      <erp-feedback id="sku-lookup-feedback" [tone]="skuStatus() === 'busy' ? 'neutral' : 'error'"
        [message]="skuStatus() ? t()(skuStatus()!) : ''" />
    </erp-section>
    <div class="empty">
      <svg lucidePackageSearch aria-hidden="true"></svg>
      <h2>{{ t()('listUnavailable') }}</h2><p>{{ t()('listHint') }}</p>
    </div>
  `
})
export class Products {
  readonly t = inject(Preferences).t;
  readonly crumbs = workspaceCrumb;
  readonly id = new FormControl('', { nonNullable: true });
  readonly idInput = viewChild.required<ElementRef<HTMLInputElement>>('idInput');
  readonly invalid = signal(false);
  readonly sku = new FormControl('', { nonNullable: true });
  readonly skuInput = viewChild.required<ElementRef<HTMLInputElement>>('skuInput');
  readonly skuStatus = signal<Message | null>(null);
  private readonly api = inject(ArticleApi);
  private readonly router = inject(Router);
  private active?: AbortController;

  constructor() {
    const destroy = inject(DestroyRef);
    destroy.onDestroy(() => this.active?.abort());
    this.sku.valueChanges.pipe(takeUntilDestroyed(destroy)).subscribe(() => {
      this.active?.abort();
      this.skuStatus.set(null);
    });
  }

  lookup() {
    const id = this.id.value.trim();
    this.invalid.set(!isArticleId(id));
    if (this.invalid()) this.idInput().nativeElement.focus();
    else void this.router.navigate(['/products', id]);
  }

  async lookupSku() {
    this.active?.abort();
    const sku = normalizeArticleSku(this.sku.value);
    if (!sku) {
      this.skuStatus.set('invalidSku');
      this.skuInput().nativeElement.focus();
      return;
    }
    const controller = this.active = new AbortController();
    const version = this.api.credentialVersion();
    this.skuStatus.set('busy');
    const result = await this.api.findBySku(sku, controller.signal);
    if (controller.signal.aborted || version !== this.api.credentialVersion()) return;
    if ('error' in result) this.skuStatus.set(result.error === 'missing' ? 'commandNoMatches' : result.error);
    else if (result.article.sku !== sku) this.skuStatus.set('executionError');
    else await this.router.navigate(['/products', result.article.id]);
  }
}

@Component({
  imports: [Feedback, PageHeading, ReactiveFormsModule, RouterLink, LucidePlus],
  template: `
    <erp-page-heading [title]="t()('register')" [description]="t()('registerIntro')" [crumbs]="crumbs" />
    <form class="card form-width" [formGroup]="form" novalidate (ngSubmit)="save()" [attr.aria-busy]="busy()">
      <fieldset [disabled]="busy()">
        <legend>{{ t()('general') }}</legend>
        <label for="sku">{{ t()('sku') }}</label>
        <input #sku id="sku" formControlName="sku" maxlength="32" required spellcheck="false"
          [attr.aria-invalid]="skuInvalid()" [attr.aria-describedby]="skuInvalid() ? 'sku-hint product-feedback' : 'sku-hint'">
        <small id="sku-hint">{{ t()('skuHint') }}</small>
        <label for="name">{{ t()('name') }}</label>
        <input #name id="name" formControlName="name" maxlength="120" required
          [attr.aria-invalid]="nameInvalid()" [attr.aria-describedby]="nameInvalid() ? 'product-feedback' : null">
        <div class="actions">
          <button type="submit" class="primary"><svg lucidePlus aria-hidden="true"></svg>{{ t()('save') }}</button>
          <a class="button" routerLink="/products">{{ t()('cancel') }}</a>
        </div>
      </fieldset>
      <erp-feedback id="product-feedback" [tone]="busy() ? 'neutral' : 'error'" [message]="status() ? t()(status()!) : ''" />
      @if (loopback && (status() === 'tokenMissing' || status() === 'denied')) {
        <a routerLink="/local-access">{{ t()('access') }}</a>
      }
    </form>
  `
})
export class ProductRegistration {
  readonly loopback = isLoopback();
  readonly t = inject(Preferences).t;
  readonly crumbs = productsCrumb;
  readonly status = signal<Message | null>(null);
  readonly busy = signal(false);
  readonly sku = viewChild<ElementRef<HTMLInputElement>>('sku');
  readonly name = viewChild.required<ElementRef<HTMLInputElement>>('name');
  private readonly api = inject(ArticleApi);
  private readonly router = inject(Router);
  readonly form = new FormGroup({
    sku: new FormControl('', { nonNullable: true, validators: [Validators.required, Validators.pattern(/^[A-Z0-9-]{1,32}$/)] }),
    name: new FormControl('', { nonNullable: true, validators: [Validators.required, Validators.maxLength(120)] })
  });

  skuInvalid() { return this.status() === 'duplicate' || (this.status() === 'invalid' && this.form.controls.sku.invalid); }
  nameInvalid() { return this.status() === 'invalid' && (!this.form.controls.name.value.trim() || this.form.controls.name.invalid); }

  async save() {
    if (this.busy()) return;
    if (this.form.invalid || !this.form.controls.name.value.trim()) {
      this.status.set('invalid');
      (this.form.controls.sku.invalid ? this.sku() : this.name())?.nativeElement.focus();
      return;
    }
    this.status.set('busy');
    this.busy.set(true);
    const result = await this.api.register(this.form.getRawValue());
    this.busy.set(false);
    if ('error' in result) {
      this.status.set(result.error);
      if (result.error === 'duplicate') {
        // The fieldset unlocks on Angular's next render; focus it after that render.
        requestAnimationFrame(() => this.sku()?.nativeElement.focus());
      }
      return;
    }
    this.form.markAsPristine();
    await this.router.navigate(['/products', result.article.id], { info: { registered: result.article } });
  }

  @HostListener('window:beforeunload', ['$event'])
  beforeUnload(event: BeforeUnloadEvent) {
    if (this.form.dirty || this.busy()) event.preventDefault();
  }
}

export const confirmLeave: CanDeactivateFn<ProductRegistration> = page =>
  !page.busy() && (!page.form.dirty || window.confirm(page.t()('discard')));

@Component({
  imports: [Feedback, PageHeading, Section, RouterLink],
  template: `
    <erp-page-heading [title]="resultArticle()?.name ?? t()('details')" [description]="resultArticle()?.sku" [crumbs]="crumbs" />
    <erp-feedback [tone]="tone()" [message]="t()(status())" />
    @if (resultArticle(); as record) {
      <erp-section [title]="t()('general')">
        <dl class="entity-details">
          <dt>{{ t()('sku') }}</dt><dd class="technical">{{ record.sku }}</dd>
          <dt>{{ t()('name') }}</dt><dd>{{ record.name }}</dd>
          <dt>{{ t()('id') }}</dt><dd class="technical">{{ record.id }}</dd>
          <dt>{{ t()('revision') }}</dt><dd>{{ formatRevision(record.revision) }}</dd>
        </dl>
      </erp-section>
    } @else if (loopback && (status() === 'tokenMissing' || status() === 'denied')) {
      <a class="button" routerLink="/local-access">{{ t()('access') }}</a>
    } @else if (tone() === 'error') {
      <a class="button" routerLink="/products">{{ t()('backToProducts') }}</a>
    }
  `
})
export class ProductDetail {
  readonly loopback = isLoopback();
  private readonly prefs = inject(Preferences);
  readonly t = this.prefs.t;
  readonly crumbs = productsCrumb;
  readonly status = signal<Message>('busy');
  readonly resultArticle = signal<Article | null>(null);
  readonly tone = signal<'neutral' | 'success' | 'error'>('neutral');
  private readonly api = inject(ArticleApi);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private active?: AbortController;

  constructor() {
    const destroy = inject(DestroyRef);
    destroy.onDestroy(() => this.active?.abort());
    this.route.paramMap.pipe(takeUntilDestroyed(destroy)).subscribe(params => {
      const id = params.get('id') ?? '';
      const info = this.router.currentNavigation()?.extras.info as { registered?: Article } | undefined;
      this.active?.abort();
      if (info?.registered?.id === id) this.show({ article: info.registered }, 'saved');
      else void this.fetch(id);
    });
  }

  formatRevision(value: number) { return new Intl.NumberFormat(this.prefs.locale()).format(value); }

  private async fetch(id: string) {
    const controller = new AbortController();
    this.active = controller;
    this.resultArticle.set(null);
    this.tone.set('neutral');
    this.status.set('busy');
    const result = await this.api.get(id, controller.signal);
    if (!controller.signal.aborted) this.show(result, 'loaded');
  }

  private show(result: Result, success: 'saved' | 'loaded') {
    if ('error' in result) {
      this.tone.set('error');
      this.status.set(result.error);
    } else {
      this.resultArticle.set(result.article);
      this.tone.set('success');
      this.status.set(success);
    }
  }
}
