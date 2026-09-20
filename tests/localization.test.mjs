import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dictionaries } from '../src/i18n.ts';
import { activityMessage, describeHdrScope, telemetryTime } from '../src/telemetryText.ts';
import { catalogNotes, launcherName } from '../src/catalogNotes.ts';

const czechCharacters = /[áčďéěíňóřšťúůýž]/i;
const catalog = JSON.parse(readFileSync(new URL('../database/hdr_games.json', import.meta.url), 'utf8'));

test('every shipped Czech catalog description has an English rendering, preserving source URLs', () => {
  for (const entry of catalog) {
    const translated = catalogNotes(entry.notes, 'en');
    assert.doesNotMatch(translated, czechCharacters, entry.name);
    assert.deepEqual(translated.match(/https?:\/\/\S+/g), (entry.notes ?? '').match(/https?:\/\/\S+/g));
    assert.equal(catalogNotes(entry.notes, 'cs'), entry.notes ?? '');
  }
});

test('unknown external catalog descriptions and game/platform names are preserved', () => {
  assert.equal(catalogNotes('A new externally supplied note', 'en'), 'A new externally supplied note');
  assert.equal(launcherName('Média', 'en'), 'Media');
  assert.equal(launcherName('Media', 'cs'), 'Média');
  assert.equal(launcherName('Steam', 'en'), 'Steam');
});

test('existing activity messages rerender in the selected language, not the event language', () => {
  for (const message of [
    { kind: 'init_system' }, { kind: 'init_detect' }, { kind: 'hdr_active' },
    { kind: 'game_hdr', appName: 'My game' }, { kind: 'sdr' }, { kind: 'mixed' },
  ]) {
    const english = activityMessage(message, dictionaries.en);
    assert.doesNotMatch(english, czechCharacters);
    assert.notEqual(english, activityMessage(message, dictionaries.cs));
  }
});

test('scan count, Alt+Tab badge, and legacy relative timestamps honor language', () => {
  assert.equal(dictionaries.en.scanModalSelected(3), '3 selected');
  assert.doesNotMatch(dictionaries.en.settingsNoFlicker, czechCharacters);
  assert.equal(telemetryTime('Včera', 'en', dictionaries.en), 'Yesterday');
  assert.equal(telemetryTime('Dnes', 'en', dictionaries.en), 'Today');
  assert.equal(telemetryTime('14:27', 'en', dictionaries.en), '14:27');
  assert.equal(
    telemetryTime('2026-04-03T18:04:00', 'en', dictionaries.en),
    new Intl.DateTimeFormat('en', { hour: '2-digit', minute: '2-digit' }).format(new Date('2026-04-03T18:04:00')),
  );
});

test('Czech and English dictionary keys remain aligned', () => {
  assert.deepEqual(Object.keys(dictionaries.en).sort(), Object.keys(dictionaries.cs).sort());
});

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
