import { describe, expect, it } from 'vitest';
import { ALL_MODULES, planModules, type ModuleKey } from './modules';

describe('planModules', () => {
  it('hides nothing by default', () => {
    expect(planModules(ALL_MODULES, 2)).toEqual({
      includeEffects: true,
      hidden: [],
      visible: ['imageControl', 'framing', 'guides', 'dateStamp'],
    });
  });

  it('turns Image Control off by leaving the effects out', () => {
    const enabled = new Set<ModuleKey>(['framing', 'dateStamp']);
    expect(planModules(enabled, 2)).toEqual({
      includeEffects: false,
      hidden: ['guides'],
      visible: ['framing', 'dateStamp'],
    });
  });

  it('shows Image Control only with effects', () => {
    expect(planModules(new Set<ModuleKey>(['imageControl']), 0).visible).toEqual([]);
    expect(planModules(new Set<ModuleKey>(['imageControl']), 1).visible).toEqual(['imageControl']);
  });

  it('hides every optional module when only Image Control is on', () => {
    expect(planModules(new Set<ModuleKey>(['imageControl']), 1).hidden).toEqual([
      'framing',
      'guides',
      'date-stamp',
    ]);
  });
});
