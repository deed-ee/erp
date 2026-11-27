import { computed, effect, Injectable, signal } from '@angular/core';
import { messages, type Message } from './messages';

function preference<const T extends string>(key: string, values: readonly T[], fallback: T) {
  const storageKey = `erp4us.ui.${key}`;
  let initial = fallback;
  try { initial = values.find(value => value === localStorage.getItem(storageKey)) ?? fallback; }
  catch { /* Preferences still work in memory when storage is disabled. */ }
  const state = signal<T>(initial);
  effect(() => {
    const value = state();
    try { localStorage.setItem(storageKey, value); }
    catch { /* Preferences still work in memory when storage is disabled. */ }
  });
  return state;
}

@Injectable({ providedIn: 'root' })
export class Preferences {
  readonly locale = preference('locale', ['en', 'de'], navigator.language.startsWith('de') ? 'de' : 'en');
  readonly theme = preference('theme', ['system', 'light', 'dark'], 'system');
  readonly density = preference('density', ['comfortable', 'compact'], 'comfortable');
  readonly secondary = preference('secondary', ['open', 'closed'], 'open');
  readonly panel = preference('panel', ['open', 'closed'], 'closed');
  readonly t = computed(() => (key: Message) => messages[this.locale()][key] ?? messages.en[key]);

  constructor() {
    effect(() => {
      const root = document.documentElement;
      root.lang = this.locale();
      root.dataset['theme'] = this.theme();
      root.dataset['density'] = this.density();
    });
  }
}
