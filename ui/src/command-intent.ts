import descriptor from '../../contracts/entities/article-v1.json';
import manifest from '../../contracts/manifests/master-data-v1.json';
import type { Article } from './article-api';
import { isArticleId, normalizeArticleSku } from './article-identifiers';
import type { Message } from './messages';

const find = descriptor.operations.findBySku;
const get = descriptor.operations.get;
export const discoveryLabel = 'masterData.article.findBySku' satisfies Message;
export const commandAvailable = descriptor.specVersion === '1.0-draft'
  && manifest.protocolVersion === '1.0-draft' && manifest.capabilities.includes(descriptor.entity)
  && find.kind === 'query' && get.kind === 'query' && find.labelKey === discoveryLabel
  && find.operationId === 'findArticleBySku' && find.discovery.parameter === 'sku'
  && find.discovery.authorizationOperationId === 'discoverArticleSkuLookup' && get.operationId === 'getArticle';
const phrases = Object.values(find.discovery.phrases).flat().sort((a, b) => b.length - a.length);
export const commandExample = (locale: 'en' | 'de') => find.discovery.phrases[locale][0];
export const maxCommandLength = 180;

type Lookup =
  | { operationId: 'getArticle'; filter: { id: string } }
  | { operationId: 'findArticleBySku'; filter: { sku: string } };
type Resolution = { kind: 'lookup'; lookup: Lookup }
  | { kind: 'clarificationRequired' | 'unsupportedQuery' };

export type CommandResult = {
  readonly schemaVersion: 1;
  readonly lookup: Lookup;
} & (
  | { status: 'success'; article: Article }
  | { status: 'noMatches' | 'unauthorized' | 'serviceUnavailable' | 'executionError'; message: Message }
);

export function normalizeCommand(value: string) {
  return value.trim().replace(/\s+/gu, ' ');
}

export function resolveCommand(value: string): Resolution {
  if (!commandAvailable || value.length > maxCommandLength) return { kind: 'unsupportedQuery' };
  const text = normalizeCommand(value).replace(/\?$/u, '').trimEnd();
  const lower = text.toLowerCase();
  if (isArticleId(text)) {
    return { kind: 'lookup', lookup: { operationId: 'getArticle', filter: { id: lower } } };
  }
  const phrase = phrases.find(candidate => lower === candidate || lower.startsWith(`${candidate} `));
  const sku = normalizeArticleSku(phrase ? text.slice(phrase.length) : text);
  if (sku) {
    return { kind: 'lookup', lookup: { operationId: 'findArticleBySku', filter: { sku } } };
  }
  return { kind: !text || phrase ? 'clarificationRequired' : 'unsupportedQuery' };
}
