import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import {
  defaultVisualPreferences, visualThemePresets, findThemePreset,
  getVisualPreferenceKey, getCustomVisualThemesKey, loadVisualPreferences,
  saveVisualPreferences, loadCustomVisualThemes, applyVisualPreferences,
} from '../../src/visual-core/theme.ts';

let storage, properties, attributes;
beforeEach(() => {
  storage = new Map(); properties = new Map(); attributes = new Map();
  globalThis.window = { localStorage: {
    getItem: key => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value),
  } };
  globalThis.document = { documentElement: {
    setAttribute: (key, value) => attributes.set(key, value),
    style: { setProperty: (key, value) => properties.set(key, value), removeProperty: key => properties.delete(key) },
  } };
});

test('new users receive ARGWS light without writing or migrating saved preferences', () => {
  assert.deepEqual(loadVisualPreferences('new'), { mode: 'light', preset: 'argws-manifestador', density: 'compact' });
  assert.equal(storage.size, 0);
  for (const raw of ['{broken', 'null']) {
    storage.set(getVisualPreferenceKey('new'), raw);
    assert.deepEqual(loadVisualPreferences('new'), defaultVisualPreferences);
    assert.equal(storage.get(getVisualPreferenceKey('new')), raw);
  }
});

test('all 13 previous themes, dark mode and densities survive round trip per user', () => {
  const oldIds = ['corporate-blue', 'sky-clean', 'financial-green', 'modern-purple', 'operational-orange', 'executive-graphite', 'classic-blue-dark', 'ocean-blue', 'emerald-dark', 'purple-dark', 'graphite-dark', 'orange-dark', 'dark-navy'];
  assert.equal(visualThemePresets.length, 14);
  for (const preset of oldIds) {
    assert.equal(findThemePreset(preset).id, preset);
    for (const density of ['comfortable', 'compact', 'dense']) {
      const prefs = { preset, mode: 'dark', density };
      saveVisualPreferences('existing', prefs);
      assert.deepEqual(loadVisualPreferences('existing'), prefs);
      assert.equal(loadVisualPreferences('other').preset, 'argws-manifestador');
    }
  }
});

test('custom selection and incomplete legacy palette keep original defaults', () => {
  storage.set(getCustomVisualThemesKey('existing'), JSON.stringify([{ id: 'mine', name: 'Meu tema', colors: { primary: '#123456' }, typography: { fontFamily: 'Arial', fontScale: 1.1 } }]));
  const custom = loadCustomVisualThemes('existing');
  assert.equal(custom[0].colors.primarySoft, '#dbeafe');
  assert.equal(custom[0].colors.accent, '#0891b2');
  assert.equal(custom[0].typography.fontFamily, 'Arial');
  saveVisualPreferences('existing', { preset: 'custom:mine', mode: 'dark', density: 'dense' });
  assert.equal(findThemePreset(loadVisualPreferences('existing').preset, custom).id, 'custom:mine');
  assert.deepEqual(loadCustomVisualThemes('other'), []);
});

test('theme switches clear stale custom text colors and restore mode/palette/density', () => {
  const custom = [{ ...findThemePreset('corporate-blue'), id: 'custom:mine', custom: true, typography: { fontFamily: 'Arial', textColor: '#123456', mutedColor: '#234567' } }];
  applyVisualPreferences({ ...defaultVisualPreferences, preset: 'custom:mine' }, custom);
  assert.equal(properties.get('--text-color'), '#123456');
  assert.equal(properties.get('--app-font-family'), 'Arial');
  applyVisualPreferences(defaultVisualPreferences);
  assert.equal(properties.has('--text-color'), false);
  assert.equal(properties.has('--text-muted'), false);
  assert.equal(properties.get('--app-font-family'), 'Inter, Segoe UI, Arial, sans-serif');
  assert.equal(properties.get('--primary'), '#334E7D');
  applyVisualPreferences({ ...defaultVisualPreferences, mode: 'dark', density: 'dense' });
  assert.equal(attributes.get('data-theme'), 'dark');
  assert.equal(properties.get('--app-font-family'), 'Inter, Segoe UI, Arial, sans-serif');
  assert.equal(attributes.get('data-density'), 'dense');
  assert.equal(properties.get('--primary-soft'), '#243247');
  applyVisualPreferences({ ...defaultVisualPreferences, preset: 'corporate-blue' });
  assert.equal(properties.get('--primary'), '#2563eb');
  assert.equal(properties.get('--primary-soft'), '#dbeafe');
  assert.doesNotMatch(properties.get('--app-font-family'), /ARGWS/);
});

function contrast(a, b) {
  const lum = hex => {
    const rgb = hex.slice(1).match(/../g).map(v => parseInt(v, 16) / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4);
    return rgb[0] * .2126 + rgb[1] * .7152 + rgb[2] * .0722;
  };
  const x = lum(a), y = lum(b);
  return (Math.max(x, y) + .05) / (Math.min(x, y) + .05);
}
test('brand text/button pairs meet 4.5:1 in light and dark modes', () => {
  const preset = findThemePreset('argws-manifestador');
  for (const bg of ['#FFFFFF', '#F5F6F8', preset.colors.primarySoft]) {
    assert.ok(contrast(preset.colors.primary, bg) >= 4.5);
  }
  assert.ok(contrast('#FFFFFF', preset.colors.primaryHover) >= 4.5);
  applyVisualPreferences({ ...defaultVisualPreferences, mode: 'dark' });
  for (const token of ['--adaptive-dark-page-bg', '--adaptive-dark-surface-bg', '--adaptive-dark-card-bg', '--adaptive-dark-muted-bg', '--primary-soft']) {
    assert.ok(contrast('#AFC6EE', properties.get(token)) >= 4.5, token);
    assert.ok(contrast('#f8fafc', properties.get(token)) >= 4.5, token);
  }
});
