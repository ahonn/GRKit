import { describe, expect, it } from 'vitest';
import { downloadProps, importProps } from './analytics';
import { CAMERAS } from './cameras';
import { planModules, type ModuleKey } from './modules';

const camera = (key: string) => CAMERAS.find((item) => item.key === key)!;

describe('downloadProps', () => {
  it('gives the camera, the effect count, the language, and each module', () => {
    const gr3 = camera('gr3');
    const enabled = new Set<ModuleKey>(['filmLook', 'framing']);
    const plan = planModules(enabled, 2, gr3.support!.modules);
    expect(downloadProps(gr3, plan, 2, 'zh-Hans')).toEqual({
      camera: 'gr3',
      effects: '2',
      language: 'zh-Hans',
      filmLook: 'on',
      framing: 'on',
      guides: 'off',
      dateStamp: 'off',
      grain: 'off',
      recipes: 'off',
    });
  });

  it('gives Image Control as off when the package has no effects', () => {
    const gr3 = camera('gr3');
    const plan = planModules(new Set<ModuleKey>(['filmLook', 'dateStamp']), 0, gr3.support!.modules);
    expect(downloadProps(gr3, plan, 0, 'en')).toMatchObject({ filmLook: 'off', dateStamp: 'on' });
  });

  it('leaves out the modules that the site does not offer for the camera', () => {
    const gr4m = camera('gr4m');
    const support = gr4m.support!;
    const plan = planModules(new Set(support.modules), 0, support.modules);
    const props = downloadProps(gr4m, plan, 0, 'en');
    expect(props.camera).toBe('gr4m');
    expect(props).not.toHaveProperty('filmLook');
    expect(props.framing).toBe('on');
  });
});

describe('importProps', () => {
  it('gives the format, the input, the choice, and the grade, and no file name', () => {
    const props = importProps('My Look.CUBE', 'vlog', true, {
      result: 'ok',
      grade: 'B',
      warnings: 2,
    });
    expect(props).toEqual({
      format: 'cube',
      input: 'vlog',
      choice: 'manual',
      result: 'ok',
      grade: 'B',
      warnings: 'yes',
    });
    expect(Object.values(props).join(' ')).not.toContain('My Look');
  });

  it('gives a failed import without a grade', () => {
    expect(importProps('look.tif', 'standard', false, { result: 'failed' })).toEqual({
      format: 'tiff',
      input: 'standard',
      choice: 'auto',
      result: 'failed',
    });
    expect(importProps('look', 'standard', false, { result: 'failed' }).format).toBe('other');
  });
});
