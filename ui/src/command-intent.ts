import descriptor from '../../contracts/entities/article-v1.json';
import manifest from '../../contracts/manifests/master-data-v1.json';
import type { ArticleLookup } from './article-lookup';
import { isArticleId, isArticleName, normalizeArticleSku, type ArticleLookupKind } from './article-identifiers';
import type { Message } from './messages';

const descriptors = { sku: descriptor.operations.findBySku, name: descriptor.operations.findByName };
const get = descriptor.operations.get;
export const discoveryLabels = {
  sku: 'masterData.article.findBySku', name: 'masterData.article.findByName'
} as const satisfies Record<ArticleLookupKind, Message>;
const supported = {
  sku: ['findArticleBySku', 'discoverArticleSkuLookup'],
  name: ['findArticlesByName', 'discoverArticleNameLookup']
} as const;
const trusted = descriptor.specVersion === '1.0-draft'
  && manifest.protocolVersion === '1.0-draft' && manifest.capabilities.includes(descriptor.entity)
  && get.kind === 'query' && get.operationId === 'getArticle';
export const queryOptions = (['sku', 'name'] as const).filter(kind => {
  const operation = descriptors[kind];
  const [operationId, authorizationOperationId] = supported[kind];
  return trusted && operation.kind === 'query' && operation.labelKey === discoveryLabels[kind]
    && operation.operationId === operationId && operation.discovery.authorizationOperationId === authorizationOperationId
    && operation.discovery.parameter === kind && operation.permission === get.permission;
});
export const commandAvailable = queryOptions.length > 0;
const phrases = queryOptions.flatMap(kind => Object.values(descriptors[kind].discovery.phrases).flat()
  .map(phrase => ({ kind, phrase }))).sort((a, b) => b.phrase.length - a.phrase.length)
  .map(({ kind, phrase }) => ({
    kind, prefix: new RegExp(`^${phrase.split(' ').map(word => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
      .join('\\s+')}(?=\\s|$)`, 'iu')
  }));
export const commandExample = (kind: ArticleLookupKind, locale: 'en' | 'de') =>
  descriptors[kind].discovery.phrases[locale][0];
// A 120-code-point name may occupy 240 UTF-16 code units before its intent prefix.
export const maxCommandLength = 320;

type Resolution = { kind: 'lookup'; lookup: ArticleLookup }
  | { kind: 'clarificationRequired'; parameter?: 'name' }
  | { kind: 'unsupportedQuery' };

export function normalizeCommand(value: string) {
  return value.trim().replace(/\s+/gu, ' ');
}

export function resolveCommand(value: string): Resolution {
  if (!commandAvailable || value.length > maxCommandLength) return { kind: 'unsupportedQuery' };
  const source = value.trim();
  const matched = phrases.map(({ kind, prefix }) => ({ kind, match: prefix.exec(source) }))
    .find(item => item.match !== null);
  if (matched?.kind === 'name' && matched.match) {
    const raw = source.slice(matched.match[0].length).trim();
    const quoted = raw.startsWith('"');
    const complete = !quoted || (raw.length >= 2 && raw.endsWith('"'));
    const name = quoted ? raw.slice(1, -1) : raw;
    return complete && isArticleName(name)
      ? { kind: 'lookup', lookup: { operationId: 'findArticlesByName', filter: { name } } }
      : { kind: 'clarificationRequired', parameter: 'name' };
  }
  const text = source.replace(/\?$/u, '').trimEnd();
  const lower = text.toLowerCase();
  if (isArticleId(text)) {
    return { kind: 'lookup', lookup: { operationId: 'getArticle', filter: { id: lower } } };
  }
  const sku = normalizeArticleSku(matched?.match ? text.slice(matched.match[0].length) : text);
  if (sku && queryOptions.includes('sku')) {
    return { kind: 'lookup', lookup: { operationId: 'findArticleBySku', filter: { sku } } };
  }
  return { kind: !text || matched ? 'clarificationRequired' : 'unsupportedQuery' };
}
