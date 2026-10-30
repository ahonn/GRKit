import { describe, expect, it } from 'vitest';
import { ALL_MODULES, planModules, type ModuleKey } from './modules';

describe('planModules', () => {
  it('adds every module by default', () => {
    expect(planModules(ALL_MODULES, 2)).toEqual({
      includeEffects: true,
      modules: ['FILMLOOK', 'FRAMING', 'GUIDES', 'DATE', 'GRAIN', 'RECIPES'],
      visible: ['filmLook', 'framing', 'guides', 'dateStamp', 'grain', 'recipes'],
    });
  });

  it('turns Image Control off by leaving out its module and the effects', () => {
    const enabled = new Set<ModuleKey>(['framing', 'dateStamp']);
    expect(planModules(enabled, 2)).toEqual({
      includeEffects: false,
      modules: ['FRAMING', 'DATE'],
      visible: ['framing', 'dateStamp'],
    });
  });

  it('adds Image Control only with effects', () => {
    expect(planModules(new Set<ModuleKey>(['filmLook']), 0)).toEqual({
      includeEffects: true,
      modules: [],
      visible: [],
    });
    expect(planModules(new Set<ModuleKey>(['filmLook']), 1).modules).toEqual(['FILMLOOK']);
  });

  it('plans only the modules that the site offers for the camera', () => {
    const offered: ModuleKey[] = ['framing', 'guides', 'dateStamp', 'grain'];
    expect(planModules(ALL_MODULES, 2, offered)).toEqual({
      includeEffects: false,
      modules: ['FRAMING', 'GUIDES', 'DATE', 'GRAIN'],
      visible: ['framing', 'guides', 'dateStamp', 'grain'],
    });
  });
});
