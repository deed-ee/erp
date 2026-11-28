import type { Article, ArticleApi, ArticleNameMatches } from './article-api';
import type { Message } from './messages';

export type ArticleLookup =
  | { operationId: 'getArticle'; filter: { id: string } }
  | { operationId: 'findArticleBySku'; filter: { sku: string } }
  | { operationId: 'findArticlesByName'; filter: { name: string } };

export type ArticleLookupResult = {
  readonly schemaVersion: 1;
  readonly lookup: ArticleLookup;
} & (
  | { status: 'success'; article: Article }
  | ({ status: 'clarificationRequired' } & ArticleNameMatches)
  | { status: 'noMatches' | 'unauthorized' | 'serviceUnavailable' | 'executionError'; message: Message }
);

export async function executeArticleLookup(
  api: ArticleApi, lookup: ArticleLookup, signal: AbortSignal
): Promise<ArticleLookupResult> {
  const response = lookup.operationId === 'getArticle'
    ? await api.get(lookup.filter.id, signal)
    : lookup.operationId === 'findArticleBySku'
      ? await api.findBySku(lookup.filter.sku, signal)
      : await api.findByName(lookup.filter.name, signal);
  const base = { schemaVersion: 1, lookup } as const;
  if ('error' in response) {
    switch (response.error) {
      case 'denied':
      case 'tokenMissing': return { ...base, status: 'unauthorized', message: response.error };
      case 'missing': return { ...base, status: 'noMatches', message: 'commandNoMatches' };
      case 'unavailable': return { ...base, status: 'serviceUnavailable', message: response.error };
      default: return { ...base, status: 'executionError', message: 'executionError' };
    }
  }
  if ('matches' in response) {
    if (!response.matches.length) return { ...base, status: 'noMatches', message: 'commandNoMatches' };
    if (response.matches.length > 1) return { ...base, status: 'clarificationRequired', ...response };
    return { ...base, status: 'success', article: response.matches[0] };
  }
  const matches = lookup.operationId === 'getArticle'
    ? response.article.id.toLowerCase() === lookup.filter.id
    : lookup.operationId === 'findArticleBySku' && response.article.sku === lookup.filter.sku;
  return matches ? { ...base, status: 'success', article: response.article }
    : { ...base, status: 'executionError', message: 'executionError' };
}
