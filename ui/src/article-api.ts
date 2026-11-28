import { computed, Injectable, signal } from '@angular/core';
import { isArticleId, isArticleSku } from './article-identifiers';
import type { Message } from './messages';

export interface Registration { sku: string; name: string }
export interface Article extends Registration { id: string; revision: number }
export type Result = { article: Article } | { error: Message };
export const isLoopback = () => ['localhost', '127.0.0.1', '[::1]'].includes(location.hostname);
const skuLookup = '/v1/articles:find-by-sku';

@Injectable({ providedIn: 'root' })
export class ArticleApi {
  private readonly credential = signal('');
  private session = new AbortController();
  private readonly version = signal(0);
  readonly credentialVersion = this.version.asReadonly();
  readonly hasCredential = computed(() => Boolean(this.credential()));

  setCredential(value: string) {
    if (!isLoopback() || !value.trim()) return false;
    this.replaceCredential(value.trim());
    return true;
  }

  clearCredential() { this.replaceCredential(''); }

  private replaceCredential(value: string) {
    this.session.abort();
    this.session = new AbortController();
    this.credential.set(value);
    this.version.update(previous => previous + 1);
  }
  register(article: Registration) { return this.request('/v1/articles:register', article); }
  get(id: string, signal?: AbortSignal): Promise<Result> {
    return isArticleId(id) ? this.request(`/v1/articles/${id}`, undefined, signal) : Promise.resolve({ error: 'invalidId' });
  }

  findBySku(sku: string, signal?: AbortSignal): Promise<Result> {
    return isArticleSku(sku)
      ? this.request(`${skuLookup}?${new URLSearchParams({ sku })}`, undefined, signal)
      : Promise.resolve({ error: 'invalidSku' });
  }

  async canFindBySku(signal: AbortSignal): Promise<boolean> {
    if (!isLoopback() || !this.hasCredential()) return false;
    try { return (await this.send(skuLookup, 'HEAD', signal)).status === 204; }
    catch { return false; }
  }

  private send(path: string, method: 'HEAD' | 'GET' | 'POST', signal?: AbortSignal, body?: Registration) {
    const signals = [this.session.signal, AbortSignal.timeout(15000)];
    if (signal) signals.push(signal);
    return fetch(path, {
      method,
      headers: {
        Authorization: `Bearer ${this.credential()}`,
        'X-Erp4us-Tenant': 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5',
        'X-Erp4us-Organization': '3e793fb5-75a7-45f6-8f66-45a58fcec674',
        ...(body ? { 'Content-Type': 'application/json' } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      redirect: 'error', cache: 'no-store', signal: AbortSignal.any(signals)
    });
  }

  private async request(path: string, body?: Registration, signal?: AbortSignal): Promise<Result> {
    if (!isLoopback() || !this.credential()) return { error: 'tokenMissing' };
    try {
      const response = await this.send(path, body ? 'POST' : 'GET', signal, body);
      if (!response.ok) {
        const errors: Partial<Record<number, Message>> = {
          400: body ? 'invalid' : path.startsWith(skuLookup) ? 'invalidSku' : 'invalidId',
          401: 'denied', 403: 'denied', 404: 'missing', 409: 'duplicate', 422: 'invalid', 503: 'unavailable'
        };
        return { error: errors[response.status] ?? 'executionError' };
      }
      const article: Partial<Article> | null = await response.json().catch(() => null);
      if (!article || typeof article.id !== 'string' || !isArticleId(article.id) ||
          typeof article.sku !== 'string' || !isArticleSku(article.sku) || typeof article.name !== 'string' ||
          typeof article.revision !== 'number' || !Number.isSafeInteger(article.revision) ||
          article.revision < 1) return { error: 'executionError' };
      return { article: { id: article.id, sku: article.sku, name: article.name, revision: article.revision } };
    } catch {
      return { error: 'unavailable' };
    }
  }
}
