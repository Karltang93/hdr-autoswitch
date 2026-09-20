import { test } from 'node:test';
import assert from 'node:assert/strict';
import { manualControlAvailable, manualScopeAvailable, monitorMode, monitorReady, scopeVisuals } from '../src/displayState.ts';
import { dictionaries } from '../src/i18n.ts';

const monitor = (overrides = {}) => ({
  id: 'display', device_path: 'display', name: 'Display',
  identity_status: 'ready', identity_error: null, is_selected: false,
  adapter_id_low: 1, adapter_id_high: 0, target_id: 1,
  is_hdr_supported: true, is_hdr_enabled: false, hdr_state_known: true,
  state_error: null, is_primary: true,
  ...overrides,
});

test('manual admission follows backend authority, not automatic readiness or saved settings', () => {
  for (const target_status of ['automation_paused', 'needs_confirmation', 'disconnected', 'ready']) {
    assert.equal(manualControlAvailable({
      manual_control: { status: 'available' }, target_status, inventory_stale: false,
    }, true), true);
  }
  for (const reason of ['controller conflict', 'shutting down', 'authority unavailable']) {
    assert.equal(manualControlAvailable({
      manual_control: { status: 'blocked', reason }, inventory_stale: false,
    }, true), false);
  }
  assert.equal(manualControlAvailable({
    manual_control: { status: 'available' }, inventory_stale: true,
  }, true), false);
  assert.equal(manualControlAvailable({
    manual_control: { status: 'available' }, inventory_stale: false,
  }, false), false);
});

test('target options and manual scopes share native identity and known-state readiness', () => {
  const scope = { kind: 'monitor', device_path: 'DISPLAY', display_name: 'Display' };
  assert.equal(manualScopeAvailable(scope, [monitor()]), true);
  for (const invalid of [
    { identity_status: 'ambiguous', identity_error: null },
    { identity_status: 'identity_unavailable' },
    { identity_error: 'duplicate path' },
    { hdr_state_known: false },
    { state_error: 'read failed' },
    { is_hdr_supported: false },
    { device_path: null },
    { device_path: '' },
  ]) {
    const unavailable = monitor(invalid);
    assert.equal(monitorReady(unavailable), false);
    assert.equal(manualScopeAvailable(scope, [unavailable]), false);
    assert.equal(manualScopeAvailable({ kind: 'all' }, [unavailable]), false);
  }
  assert.equal(manualScopeAvailable({ kind: 'needs_confirmation', legacy_runtime_id: 'old' }, [monitor()]), false);
  assert.equal(manualScopeAvailable({ kind: 'all' }, []), false);
});

test('unknown display state cannot be presented as confirmed SDR or HDR', () => {
  for (const enabled of [false, true]) {
    assert.equal(monitorMode(monitor({ is_hdr_enabled: enabled, hdr_state_known: false })), 'unknown');
    assert.equal(monitorMode(monitor({ is_hdr_enabled: enabled, state_error: 'query failed' })), 'unknown');
  }
  assert.equal(monitorMode(monitor()), 'sdr');
  assert.equal(monitorMode(monitor({ is_hdr_enabled: true })), 'hdr');
});

test('all four scope modes have distinct hero, dial, badge, and indicator styles', () => {
  for (const field of ['panel', 'dial', 'badge', 'dot']) {
    assert.equal(new Set(Object.values(scopeVisuals).map((style) => style[field])).size, 4);
  }
  assert.match(scopeVisuals.mixed.panel, /amber/);
  assert.match(scopeVisuals.unknown.panel, /dashed/);
});

test('English and Czech explain manual versus automatic consent and unknown display state', () => {
  assert.match(dictionaries.en.configManualPolicy, /never changes settings or automatic HDR consent/);
  assert.match(dictionaries.cs.configManualPolicy, /nemění nastavení ani souhlas/);
  assert.match(dictionaries.en.configAcceptNative, /AUTOMATIC/);
  assert.match(dictionaries.cs.configAcceptNative, /AUTOMATICKÉ/);
  for (const language of ['en', 'cs']) {
    assert.notEqual(dictionaries[language].displaysStateUnknown, dictionaries[language].displaysSdrOnly);
  }
});
