import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { findLibraryApp, normalizeLibraryPath } from '../src/libraryState.ts';
import { manualScopeAvailable } from '../src/displayState.ts';

function fixture(query = '') {
  let invoke;
  const window = { __TAURI_INTERNALS__: { invoke: async () => null } };
  const context = vm.createContext({
    window, location: { search: query }, URLSearchParams, structuredClone,
    localStorage: { setItem() {}, removeItem() {} },
    mockWindows() {}, mockIPC(handler) { invoke = handler; }, emit: async () => {},
    findLibraryApp, normalizeLibraryPath, manualScopeAvailable,
  });
  const source = readFileSync(new URL('./ui-fixture.js', import.meta.url), 'utf8')
    .replace(/^import .*;$/gm, '').replace("await import('../src/main.tsx');", '');
  new vm.Script(source, { filename: 'ui-fixture.js' }).runInContext(context);
  return { invoke, state: window.__hdrFixture };
}

const app = (overrides = {}) => ({
  name: 'Fixture game', exe_name: 'game.exe', enabled: true, hdr_type: 'native',
  path: 'D:\\Game\\game.exe', alternate_exes: [], steam_id: null, launcher: 'Steam', ...overrides,
});

test('UI fixture scan evidence survives auto-detect off without pretending to be saved enablement', async () => {
  const { invoke } = fixture();
  const scan = await invoke('scan_installed_games');
  assert.equal(scan.games[0].is_hdr_supported, true);
  assert.equal(scan.games[0].default_selected, false);
  assert.equal(scan.games[0].evidence.status, 'verified');
  assert.equal('enabled' in scan.games[0], false);
});

test('UI fixture rejects inconsistent browse bindings and conflicting batches atomically', async () => {
  const { invoke, state } = fixture();
  const before = state.snapshot;
  await assert.rejects(invoke('add_custom_app', { app: app({ path: 'D:\\Game\\other.exe' }) }), /consistent/);
  await assert.rejects(invoke('import_detected_games', {
    expectedLibraryGeneration: before.library_generation,
    detected: [app(), app({ path: 'E:\\Other\\game.exe' })],
  }), /Select only one installation/);
  assert.deepEqual(state.snapshot, before);
});

test('UI fixture row commands enforce generation, index and path instead of executable-only deletion', async () => {
  const { invoke, state } = fixture();
  await invoke('add_custom_app', { app: app() });
  const before = state.snapshot;
  await assert.rejects(invoke('remove_app', {
    expectedLibraryGeneration: before.library_generation,
    row: { index: 0, exe_name: 'game.exe', path: 'E:\\Wrong\\game.exe' },
  }), /row changed/);
  await assert.rejects(invoke('toggle_app', {
    expectedLibraryGeneration: '0',
    row: { index: 0, exe_name: 'game.exe', path: 'D:\\Game\\game.exe' }, enabled: false,
  }), /Stale/);
  assert.deepEqual(state.snapshot, before);
  await invoke('remove_app', {
    expectedLibraryGeneration: before.library_generation,
    row: { index: 0, exe_name: 'game.exe', path: 'D:\\Game\\game.exe' },
  });
  assert.equal(state.snapshot.settings.apps.length, 0);
});

test('UI fixture catalog alias addition preserves path scoped to the retained primary', async () => {
  const { invoke, state } = fixture('?aliasMerge=1');
  const saved = state.snapshot.settings.apps[0];
  await invoke('add_custom_app', { app: app({ steam_id: '100' }) });
  const after = state.snapshot.settings.apps[0];
  assert.equal(after.exe_name, saved.exe_name);
  assert.equal(after.path, saved.path);
  assert.equal(after.enabled, true);
  assert.deepEqual(after.alternate_exes, ['game.exe']);
  assert.equal((await invoke('get_running_processes'))[0].tracked_primary, 'renderer.exe');
});

test('UI fixture surfaces controller conflict and blocks manual writes without relabeling it ready', async () => {
  const { invoke, state } = fixture('?conflict=1&mixed=1');
  const before = state.snapshot;
  const status = await invoke('get_current_status');
  assert.equal(status.target_status, 'controller_conflict');
  assert.equal(status.scope_hdr_state, 'unknown');
  assert.equal(status.manual_control.status, 'blocked');
  await assert.rejects(invoke('set_hdr', { scope: { kind: 'all' }, enable: true }), /controller conflict/);
  assert.deepEqual(state.snapshot, before);
});
