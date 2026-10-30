import { describe, expect, it } from 'vitest';
import { MAX_EFFECTS } from './catalog';
import { loadSelection, resolveSelection, saveSelection, selectImport } from './selection';

const ids = (count: number) => Array.from({ length: count }, (_, index) => `effect-${index}`);

describe('resolveSelection', () => {
  it('checks every catalog effect before the first change', () => {
    expect([...resolveSelection(null, ids(18))]).toEqual(ids(18));
  });

  it('checks at most the package limit by default', () => {
    expect(resolveSelection(null, ids(MAX_EFFECTS + 5)).size).toBe(MAX_EFFECTS);
  });

  it('keeps the choice of the user, also an empty one', () => {
    expect(resolveSelection(new Set(), ids(18)).size).toBe(0);
    expect([...resolveSelection(new Set(['effect-3']), ids(18))]).toEqual(['effect-3']);
  });

  it('keeps only the effects of the page, so a removed effect does not count', () => {
    const choice = new Set(['effect-3', 'lut-sky', 'gone', 'card-only']);
    expect([...resolveSelection(choice, ids(18), ['lut-sky', 'card-only'])]).toEqual([
      'effect-3',
      'lut-sky',
      'card-only',
    ]);
  });
});

/** A storage with one value, or one whose access fails. */
function storage(value: string | null, fails = false) {
  const saved = new Map<string, string>();
  if (value !== null) saved.set('grkit.selection', value);
  return {
    saved,
    getItem: (key: string) => {
      if (fails) throw new Error('blocked');
      return saved.get(key) ?? null;
    },
    setItem: (key: string, item: string) => {
      if (fails) throw new Error('blocked');
      saved.set(key, item);
    },
  };
}

describe('loadSelection and saveSelection', () => {
  it('give back the choice of the last visit, also an empty one', () => {
    const store = storage(null);
    saveSelection(store, new Set(['effect-1', 'lut-sky']));
    expect([...loadSelection(store)!]).toEqual(['effect-1', 'lut-sky']);
    saveSelection(store, new Set());
    expect(loadSelection(store)!.size).toBe(0);
  });

  it('use the default without a choice, storage, or readable value', () => {
    expect(loadSelection(storage(null))).toBeNull();
    expect(loadSelection(null)).toBeNull();
    expect(loadSelection(storage('{'))).toBeNull();
    expect(loadSelection(storage('"effect-1"'))).toBeNull();
    expect(loadSelection(storage('[1, 2]'))).toBeNull();
    expect(loadSelection(storage(JSON.stringify(['x'.repeat(40)])))).toBeNull();
    expect(loadSelection(storage('[]', true))).toBeNull();
  });

  it('ignore a storage that fails', () => {
    expect(() => saveSelection(storage(null, true), new Set(['effect-1']))).not.toThrow();
  });
});

describe('selectImport', () => {
  it('checks a close fit when the package has room', () => {
    const result = selectImport(new Set(ids(18)), ['lut'], 'B');
    expect(result.selection.has('lut')).toBe(true);
    expect(result.full).toBe(false);
  });

  it('leaves a close fit out when the package is full', () => {
    const checked = new Set(ids(MAX_EFFECTS));
    expect(selectImport(checked, ['lut'], 'A')).toEqual({ selection: checked, full: true });
  });

  it('leaves a loose fit for the user to check', () => {
    const checked = new Set(ids(2));
    expect(selectImport(checked, ['lut'], 'C')).toEqual({ selection: checked, full: false });
  });

  it('checks native looks up to the package limit', () => {
    const checked = new Set(ids(MAX_EFFECTS - 1));
    const result = selectImport(checked, ['yellow', 'green'], 'exact');
    expect(result.selection.has('yellow')).toBe(true);
    expect(result.selection.has('green')).toBe(false);
    expect(result.full).toBe(true);
  });

  it('keeps the selection of a replaced import that is checked', () => {
    const checked = new Set([...ids(MAX_EFFECTS - 1), 'yellow']);
    expect(selectImport(checked, ['yellow'], 'exact')).toEqual({ selection: checked, full: false });
  });
});
