import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dictionaries } from '../src/i18n.ts';
import { activityMessage, describeHdrScope } from '../src/telemetryText.ts';

test('mixed HDR/SDR never renders a confirmed SDR headline or activity message', () => {
  const result = describeHdrScope({
    scope_hdr_state: 'mixed', target_status: 'ready', inventory_stale: false,
  }, dictionaries.en);
  assert.equal(result.badge, 'MIXED HDR / SDR');
  assert.match(result.title, /MIXED/);
  assert.notEqual(result.title, dictionaries.en.heroSdrTitle);
  assert.match(activityMessage({ kind: 'mixed' }, dictionaries.en), /mixed HDR and SDR/);
});

test('unavailable and unverified scope stays unknown instead of appearing SDR', () => {
  for (const status of [
    { scope_hdr_state: 'sdr', target_status: 'ready', inventory_stale: true },
    { scope_hdr_state: 'sdr', target_status: 'disconnected', inventory_stale: false },
    { scope_hdr_state: 'hdr', target_status: 'outcome_unknown', inventory_stale: false },
    { scope_hdr_state: 'unknown', target_status: 'ready', inventory_stale: false },
  ]) {
    assert.equal(describeHdrScope(status, dictionaries.en).mode, 'unknown');
  }
});

test('a deferred unavailable target does not hide the verified active session scope', () => {
  const state = {
    scope_hdr_state: 'hdr', target_status: 'disconnected', inventory_stale: false,
    target_deferred: true, active_target: { kind: 'monitor', device_path: 'active', display_name: 'Active display' },
    uncertain_targets: [],
  };
  assert.equal(describeHdrScope(state, dictionaries.en).mode, 'hdr');
  assert.equal(describeHdrScope({ ...state, target_status: 'controller_conflict' }, dictionaries.en).mode, 'unknown');
  assert.equal(describeHdrScope({ ...state, uncertain_targets: ['active'] }, dictionaries.en).mode, 'unknown');
});
