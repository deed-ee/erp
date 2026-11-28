import { expect, test } from '@playwright/test';
import { articleContextLifetimeMs, createArticleNameContext, matchesFollowUpEvidence,
  resolveSkuFollowUp } from '../src/article-follow-up';
import type { ArticleLookupResult } from '../src/article-lookup';

const article = { id: 'c221256f-5b26-460a-8c0b-f40dae334255', sku: 'PART-1', name: 'Drive  housing', revision: 2 };
const result: ArticleLookupResult = { schemaVersion: 1, status: 'success', article,
  lookup: { operationId: 'findArticlesByName', filter: { name: article.name } } };
const created = 1000;
const credential = 1;
const context = createArticleNameContext(result, credential, created);

test('a SKU follow-up resolves only visible matches of a live name result', () => {
  expect(context).toEqual({
    name: article.name, credentialVersion: credential, expiresAt: created + articleContextLifetimeMs,
    members: [{ id: article.id, sku: article.sku, revision: article.revision }]
  });
  expect(createArticleNameContext({ ...result, status: 'noMatches', message: 'commandNoMatches' }, credential, created)).toBeNull();
  expect(createArticleNameContext({ ...result, lookup: { operationId: 'getArticle', filter: { id: article.id } } },
    credential, created)).toBeNull();
  for (const text of ['Only SKU PART-1?', 'nur   sku part-1']) {
    expect(resolveSkuFollowUp(text, context, credential, created).kind).toBe('lookup');
  }
  expect(resolveSkuFollowUp('Only SKU HIDDEN-1', context, credential, created).kind).toBe('commandFollowUpOutsideResults');
  expect(resolveSkuFollowUp('Only SKU PART-1 or PART-2', context, credential, created).kind).toBe('commandFollowUpNeedsSku');
  expect(resolveSkuFollowUp('Find article with SKU PART-1', context, credential, created).kind).toBe('notFollowUp');
  expect(resolveSkuFollowUp('Only SKU PART-1', context, credential + 1, created).kind).toBe('commandFollowUpNoContext');
  expect(resolveSkuFollowUp('Only SKU PART-1', context, credential, created + articleContextLifetimeMs).kind)
    .toBe('commandFollowUpNoContext');
});

test('new owner evidence must still match the earlier Article identity and name', () => {
  const followUp = resolveSkuFollowUp('Only SKU PART-1', context, credential, created);
  if (followUp.kind !== 'lookup') throw new Error('Expected a match for the previously shown SKU.');
  expect(matchesFollowUpEvidence(article, followUp)).toBe(true);
  expect(matchesFollowUpEvidence({ ...article, revision: 3 }, followUp)).toBe(true);
  expect(matchesFollowUpEvidence({ ...article, revision: 1 }, followUp)).toBe(false);
  expect(matchesFollowUpEvidence({ ...article, name: 'Different product name' }, followUp)).toBe(false);
  expect(matchesFollowUpEvidence({ ...article, id: 'b65b1de0-331a-4e6a-96d0-40d2f459311f' }, followUp)).toBe(false);
});
