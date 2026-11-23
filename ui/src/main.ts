import { Component, computed, signal } from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
import { FormControl, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { LucideLanguages, LucidePlus, LucideSearch } from '@lucide/angular';
import { messages, type Message } from './messages';

interface Article { id: string; sku: string; name: string; revision: number }

@Component({
  selector: 'app-root',
  imports: [ReactiveFormsModule, LucideLanguages, LucidePlus, LucideSearch],
  templateUrl: './workspace.html'
})
class Workspace {
  readonly locale = signal<'de' | 'en'>('de');
  readonly text = computed(() => messages[this.locale()]);
  readonly token = new FormControl('', { nonNullable: true });
  readonly form = new FormGroup({
    sku: new FormControl('', { nonNullable: true, validators: [Validators.required, Validators.pattern(/^[A-Z0-9-]{1,32}$/)] }),
    name: new FormControl('', { nonNullable: true, validators: [Validators.required, Validators.maxLength(120)] })
  });
  readonly id = new FormControl('', { nonNullable: true });
  readonly article = signal<Article | null>(null);
  readonly status = signal<Message | null>(null);
  readonly busy = signal(false);

  switchLanguage() {
    this.locale.update(l => l === 'de' ? 'en' : 'de');
    document.documentElement.lang = this.locale();
  }

  async submit(mode: 'register' | 'get') {
    if (this.busy()) return;
    this.article.set(null);
    if (!this.token.value.trim()) return this.status.set('tokenMissing');
    const register = mode === 'register';
    const id = this.id.value.trim();
    if (register && (this.form.invalid || !this.form.controls.name.value.trim())) return this.status.set('invalid');
    if (!register && !/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(id)) return this.status.set('invalidId');
    this.status.set('busy');
    this.busy.set(true);
    try {
      const result = await fetch(register ? '/v1/articles:register' : `/v1/articles/${id}`, {
        method: register ? 'POST' : 'GET',
        headers: { Authorization: `Bearer ${this.token.value.trim()}`, ...(register ? { 'Content-Type': 'application/json' } : {}) },
        body: register ? JSON.stringify(this.form.getRawValue()) : undefined,
        redirect: 'error', cache: 'no-store', signal: AbortSignal.timeout(15000)
      });
      if (!result.ok) {
        const errors: Partial<Record<number, Message>> = {401: 'denied', 403: 'denied', 404: 'missing', 409: 'duplicate', 422: 'invalid'};
        this.status.set(errors[result.status] ?? 'unavailable');
        return;
      }
      const article: Article = await result.json();
      this.article.set(article);
      this.id.setValue(article.id);
      this.status.set(register ? 'saved' : 'loaded');
    } catch {
      this.status.set('unavailable');
    } finally {
      this.busy.set(false);
    }
  }
}

bootstrapApplication(Workspace).catch(() => console.error('Workspace startup failed.'));
