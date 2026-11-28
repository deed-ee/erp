import type { Article } from './article-api';
import { normalizeArticleSku } from './article-identifiers';
import type { ArticleLookup, ArticleLookupResult } from './article-lookup';

export const articleContextLifetimeMs = 120_000;
type ArticleReference = Readonly<Pick<Article, 'id' | 'sku' | 'revision'>>;
export interface ArticleNameContext {
  readonly name: string;
  readonly credentialVersion: number;
  readonly expiresAt: number;
  readonly members: readonly ArticleReference[];
}

export type SkuFollowUp =
  | { kind: 'lookup'; context: ArticleNameContext; member: ArticleReference;
      lookup: Extract<ArticleLookup, { operationId: 'findArticleBySku' }> }
  | { kind: 'notFollowUp' }
  | { kind: 'commandFollowUpNoContext' | 'commandFollowUpNeedsSku' | 'commandFollowUpOutsideResults' };

export function createArticleNameContext(result: ArticleLookupResult, credentialVersion: number, now: number): ArticleNameContext | null {
  if (result.lookup.operationId !== 'findArticlesByName') return null;
  const articles = result.status === 'success' ? [result.article]
    : result.status === 'clarificationRequired' ? result.matches : [];
  if (!articles.length) return null;
  return {
    name: result.lookup.filter.name,
    credentialVersion,
    expiresAt: now + articleContextLifetimeMs,
    members: articles.map(({ id, sku, revision }) => ({ id, sku, revision }))
  };
}

export function hasActiveArticleContext(
  context: ArticleNameContext | null, credentialVersion: number, now: number
): context is ArticleNameContext {
  return context !== null && context.credentialVersion === credentialVersion && now < context.expiresAt;
}

export function resolveSkuFollowUp(input: string, context: ArticleNameContext | null, version: number, now: number): SkuFollowUp {
  const text = input.trim().replace(/\?$/u, '').trimEnd();
  const prefix = /^(?:only|nur)\s+sku(?=\s|$)/iu.exec(text);
  if (!prefix) return { kind: 'notFollowUp' };
  if (!hasActiveArticleContext(context, version, now)) return { kind: 'commandFollowUpNoContext' };
  const sku = normalizeArticleSku(text.slice(prefix[0].length));
  if (!sku) return { kind: 'commandFollowUpNeedsSku' };
  const member = context.members.find(article => article.sku === sku);
  return member ? { kind: 'lookup', lookup: { operationId: 'findArticleBySku', filter: { sku } }, context, member }
    : { kind: 'commandFollowUpOutsideResults' };
}

// A follow-up uses a fresh owner read, not the previous result as the answer.
export function matchesFollowUpEvidence(article: Article, followUp: Extract<SkuFollowUp, { kind: 'lookup' }>) {
  return article.id.toLowerCase() === followUp.member.id.toLowerCase()
    && article.sku === followUp.member.sku && article.name === followUp.context.name
    && article.revision >= followUp.member.revision;
}
