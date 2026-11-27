import { computed, Injectable, signal } from '@angular/core';
import type { Message } from './messages';

export interface Registration { sku: string; name: string }
export interface Article extends Registration { id: string; revision: number }
export type Result = { article: Article } | { error: Message };
export const isArticleId = (id: string) => /^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(id);
export const isLoopback = () => ['localhost', '127.0.0.1', '[::1]'].includes(location.hostname);

interface Grant { tenant_id: string; organization_id: string; permissions: string[] }
const fixtureScope = {
  tenant_id: 'ddcae139-c1e8-47d2-8b48-349cb6f30ce5',
  organization_id: '3e793fb5-75a7-45f6-8f66-45a58fcec674',
};

@Injectable({ providedIn: 'root' })
export class ArticleApi {
  private readonly credential = signal('');
  private readonly verifiedGrants = signal<Grant[]>([]);
  private readonly selected = signal(0);
  private pending?: AbortController;
  readonly hasCredential = computed(() => Boolean(this.credential()));
  readonly grants = this.verifiedGrants.asReadonly();
  readonly grant = computed(() => this.grants()[this.selected()]);
  readonly selectedIndex = this.selected.asReadonly();
  readonly canPlan = computed(() => this.grant()?.permissions.includes('manufacturing.plan.read') ?? false);

  setCredential(value: string) {
    if (!isLoopback() || !value.trim()) return false;
    this.clearCredential();
    this.credential.set(value.trim());
    void this.readGrants(value.trim());
    return true;
  }

  clearCredential() {
    this.pending?.abort();
    this.credential.set('');
    this.verifiedGrants.set([]);
    this.selected.set(0);
  }
  selectGrant(index: number) {
    if (this.grants()[index]) this.selected.set(index);
  }
  scopeLabel(grant: Grant) {
    return grant.tenant_id === fixtureScope.tenant_id && grant.organization_id === fixtureScope.organization_id
      ? 'NovaMotion' : `${grant.tenant_id.slice(0, 8)} / ${grant.organization_id.slice(0, 8)}`;
  }
  requestHeaders(): Record<string, string> {
    const scope = this.grant() ?? fixtureScope;
    return {
      Authorization: `Bearer ${this.credential()}`,
      'X-Erp4us-Tenant': scope.tenant_id,
      'X-Erp4us-Organization': scope.organization_id,
    };
  }
  private async readGrants(token: string) {
    const pending = new AbortController();
    this.pending = pending;
    try {
      const response = await fetch('/v1/session', {
        headers: { Authorization: `Bearer ${token}` }, cache: 'no-store', redirect: 'error',
        signal: AbortSignal.any([pending.signal, AbortSignal.timeout(5000)]),
      });
      if (!response.ok) return;
      const principal: { grants?: Grant[] } = await response.json();
      if (!pending.signal.aborted && principal && Array.isArray(principal.grants)) {
        this.verifiedGrants.set(principal.grants.filter(grant => grant && isArticleId(grant.tenant_id)
          && isArticleId(grant.organization_id) && Array.isArray(grant.permissions)
          && grant.permissions.every(permission => typeof permission === 'string')));
      }
    } catch {
      // No verified grants are exposed on failure. The original local fixture
      // flow may still display a request error from the owning service.
    }
  }
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
        headers: {
          ...this.requestHeaders(),
          ...(body ? { 'Content-Type': 'application/json' } : {}),
        },
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
