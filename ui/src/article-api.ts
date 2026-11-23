import { computed, Injectable, signal } from '@angular/core';
import type { Message } from './messages';

export interface Registration { sku: string; name: string }
export interface Article extends Registration { id: string; revision: number }
export type Result = { article: Article } | { error: Message };
export const isArticleId = (id: string) => /^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(id);
export const isLoopback = () => ['localhost', '127.0.0.1', '[::1]'].includes(location.hostname);

@Injectable({ providedIn: 'root' })
export class ArticleApi {
  private readonly credential = signal('');
  readonly hasCredential = computed(() => Boolean(this.credential()));

  setCredential(value: string) {
    if (!isLoopback() || !value.trim()) return false;
    this.credential.set(value.trim());
    return true;
  }

  clearCredential() { this.credential.set(''); }
  register(article: Registration) { return this.request('/v1/articles:register', article); }
  get(id: string, signal?: AbortSignal): Promise<Result> {
    return isArticleId(id) ? this.request(`/v1/articles/${id}`, undefined, signal) : Promise.resolve({ error: 'invalidId' });
  }

  private async request(path: string, body?: Registration, signal?: AbortSignal): Promise<Result> {
    if (!isLoopback() || !this.credential()) return { error: 'tokenMissing' };
    const timeLimit = AbortSignal.timeout(15000);
    try {
      const response = await fetch(path, {
        method: body ? 'POST' : 'GET',
        headers: { Authorization: `Bearer ${this.credential()}`, ...(body ? { 'Content-Type': 'application/json' } : {}) },
        body: body ? JSON.stringify(body) : undefined,
        redirect: 'error', cache: 'no-store', signal: signal ? AbortSignal.any([signal, timeLimit]) : timeLimit
      });
      if (!response.ok) {
        const errors: Partial<Record<number, Message>> = {
          400: body ? 'invalid' : 'invalidId', 401: 'denied', 403: 'denied', 404: 'missing', 409: 'duplicate', 422: 'invalid'
        };
        return { error: errors[response.status] ?? 'unavailable' };
      }
      const article: Partial<Article> | null = await response.json();
      if (!article || typeof article.id !== 'string' || !isArticleId(article.id) ||
          typeof article.sku !== 'string' || typeof article.name !== 'string' ||
          typeof article.revision !== 'number' || !Number.isSafeInteger(article.revision)) return { error: 'unavailable' };
      return { article: { id: article.id, sku: article.sku, name: article.name, revision: article.revision } };
    } catch {
      return { error: 'unavailable' };
    }
  }
}
