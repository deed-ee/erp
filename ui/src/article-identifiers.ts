export const isArticleId = (id: string) =>
  /^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(id) && id === id.trim();
export const isArticleSku = (sku: string) => /^[A-Z0-9-]{1,32}$/.test(sku) && sku === sku.trim();

export function normalizeArticleSku(value: string): string | null {
  const input = value.trim();
  return /^[A-Za-z0-9-]{1,32}$/.test(input) ? input.toUpperCase() : null;
}
