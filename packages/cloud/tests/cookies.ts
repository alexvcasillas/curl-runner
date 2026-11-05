import { vi } from 'vitest';

export type SetCall = { name: string; value: string; options: Record<string, unknown> };

/** An in-memory `cookies()` store, enough for `lib/session`. */
export function cookieJar(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  const sets: SetCall[] = [];
  const store = {
    get: vi.fn((name: string) => {
      const value = values.get(name);
      return value === undefined ? undefined : { name, value };
    }),
    set: vi.fn((name: string, value: string, options: Record<string, unknown> = {}) => {
      sets.push({ name, value, options });
      if (options.maxAge === 0) {
        values.delete(name);
      } else {
        values.set(name, value);
      }
    }),
    delete: vi.fn((arg: string | { name: string }) => {
      values.delete(typeof arg === 'string' ? arg : arg.name);
    }),
  };
  return { store, sets, values };
}
