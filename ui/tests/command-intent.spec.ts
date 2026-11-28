import { expect, test } from '@playwright/test';
import { maxCommandLength, resolveCommand } from '../src/command-intent';

test('maps both languages and direct SKUs to the same typed operation', () => {
  for (const input of [' nm-1000 ', 'Find article with SKU NM-1000?',
    'Zeige   mir den Artikel mit SKU nm-1000', 'Find product by SKU NM-1000', 'zeige produkt NM-1000']) {
    expect(resolveCommand(input)).toEqual({
      kind: 'lookup', lookup: { operationId: 'findArticleBySku', filter: { sku: 'NM-1000' } }
    });
  }
  const id = '242a290f-4bbc-438e-aefe-e3697a8bcd16';
  expect(resolveCommand(id.toUpperCase())).toEqual({
    kind: 'lookup', lookup: { operationId: 'getArticle', filter: { id } }
  });
});

test('never resolves missing, multiple or unsupported inputs to a write', () => {
  for (const input of ['', 'zeige produkt', 'Find article with SKU A or B', 'zeige produkt NM-1000/A']) {
    expect(resolveCommand(input)).toEqual({ kind: 'clarificationRequired' });
  }
  for (const input of ['create article A', 'show products with stock', 'Ä-1', 'NM/1000',
    'sku[$ne]=A', 'A'.repeat(maxCommandLength + 1), 'show productA']) {
    expect(resolveCommand(input)).toEqual({ kind: 'unsupportedQuery' });
  }
});

test('resolves exact names without changing case, Unicode, punctuation or internal spaces', () => {
  const name = ' Geh\u0061\u0308use  Type? ';
  for (const input of [`Find   article named "${name}"`, `FINDE ARTIKEL MIT BEZEICHNUNG "${name}"`]) {
    expect(resolveCommand(input)).toEqual({
      kind: 'lookup', lookup: { operationId: 'findArticlesByName', filter: { name } }
    });
  }
  expect(resolveCommand('Show articles named Gearbox')).toEqual({
    kind: 'lookup', lookup: { operationId: 'findArticlesByName', filter: { name: 'Gearbox' } }
  });
  for (const input of ['find article named', 'find article named "Unclosed', 'find article named " "',
    `find article named "${'🧰'.repeat(121)}"`, 'find article named Control\nHere']) {
    expect(resolveCommand(input)).toEqual({ kind: 'clarificationRequired', parameter: 'name' });
  }
});
