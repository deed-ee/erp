const de = {
  area: 'Stammdaten', title: 'Artikel', intro: 'Lege einen Artikel an oder rufe einen vorhandenen Datensatz über seine ID ab.',
  local: 'Lokale Demo', access: 'Demo-Zugang', token: 'Demo-Token', hint: 'Verwende den Wert ERP4US_DEMO_TOKEN aus der lokalen .env-Datei. Der Token bleibt nur bis zum Neuladen im Arbeitsspeicher.',
  language: 'English', register: 'Artikel anlegen', lookup: 'Artikel abrufen', sku: 'SKU', name: 'Bezeichnung', id: 'Artikel-ID',
  skuHint: '1–32 Zeichen: A–Z, Ziffern oder Bindestrich.', details: 'Artikeldetails', revision: 'Revision',
  saved: 'Artikel angelegt.', loaded: 'Artikel abgerufen.', busy: 'Anfrage läuft …',
  tokenMissing: 'Bitte zuerst den Demo-Token eingeben.', invalid: 'Bitte SKU und Bezeichnung prüfen.',
  invalidId: 'Bitte eine gültige Artikel-ID (UUID) eingeben.', duplicate: 'Diese SKU ist bereits vergeben. Bitte ändere die SKU.',
  denied: 'Zugriff verweigert. Bitte den Demo-Token prüfen.', missing: 'Kein Artikel mit dieser ID gefunden.',
  unavailable: 'Die Anfrage ist fehlgeschlagen. Bitte den lokalen API-Dienst und den eingegebenen Datensatz prüfen.'
};
const en: typeof de = {
  area: 'Master data', title: 'Articles', intro: 'Register an article or retrieve an existing record by its ID.',
  local: 'Local demo', access: 'Demo access', token: 'Demo token', hint: 'Use ERP4US_DEMO_TOKEN from your local .env file. The token stays in memory only until you reload.',
  language: 'Deutsch', register: 'Register article', lookup: 'Retrieve article', sku: 'SKU', name: 'Name', id: 'Article ID',
  skuHint: '1–32 characters: A–Z, digits or hyphens.', details: 'Article details', revision: 'Revision',
  saved: 'Article registered.', loaded: 'Article retrieved.', busy: 'Request in progress …',
  tokenMissing: 'Enter the demo token first.', invalid: 'Check the SKU and name.',
  invalidId: 'Enter a valid article ID (UUID).', duplicate: 'This SKU already exists. Please change the SKU.',
  denied: 'Access denied. Check your demo token.', missing: 'No article found with this ID.',
  unavailable: 'Request failed. Check the local API service and the data you entered.'
};
export const messages = { de, en };
export type Message = keyof typeof de;
