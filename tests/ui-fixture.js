import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';

const options = new URLSearchParams(location.search);
localStorage.setItem('hdr_lang', options.get('lang') === 'cs' ? 'cs' : 'en');
localStorage.removeItem('hdr_recent_games');
mockWindows('main');
const mode = options.get('mode') ?? 'ready';
const settings = {
  target_monitor: options.get('mixed') === '1' ? { kind: 'all' } : mode === 'import_available'
    ? { kind: 'needs_confirmation', legacy_runtime_id: 'old-runtime-id' }
    : { kind: 'monitor', device_path: 'missing-monitor', display_name: 'Saved gaming display' },
  alt_tab_delay_seconds: 2,
  exit_only_hdr: true,
  notifications_enabled: false,
  autostart: false,
  start_minimized: false,
  auto_detect_new_games: false,
  auto_sync_database: false,
  switch_method: mode === 'import_available' ? 'shortcut' : 'native',
  blacklist: [],
  apps: [],
};
let snapshot = {
  mode, settings, store_id: 'fixture-history', revision: '1', control_epoch: '1',
  context_token: 'fixture-context', library_generation: '1', issue: null,
  controller_issue: null, config_path: 'C:\\ISOLATED-UI-FIXTURE\\config-v2.json',
  candidates: mode === 'recovery_required'
    ? [{ id: 'validated-checkpoint', label: 'Validated fixture checkpoint' }] : [],
};
const monitors = [{
  id: 'connected-monitor', device_path: 'connected-monitor', name: 'Connected test display',
  identity_status: 'ready', identity_error: null,
  adapter_id_low: 999, adapter_id_high: 0, target_id: 23,
  is_hdr_supported: true, is_hdr_enabled: false, is_primary: true,
  hdr_state_known: true, state_error: null,
}];
if (options.get('mixed') === '1') {
  monitors[0].is_hdr_enabled = true;
  monitors.push({
    ...monitors[0], id: 'second-monitor', device_path: 'second-monitor',
    name: 'Second test display', target_id: 24, is_hdr_enabled: false, is_primary: false,
  });
}
const games = [{
  name: 'Fixture game', exe_name: 'fixture.exe', hdr_type: 'native', enabled: true,
  alternate_exes: [], launcher: 'Média',
}];
const commands = [];
let history = 1;
let failSave = options.get('failSave') === '1';

function status() {
  const target = snapshot.settings.target_monitor;
  const target_status = snapshot.mode !== 'ready' || snapshot.settings.switch_method !== 'native'
    ? 'automation_paused'
    : target.kind === 'needs_confirmation' ? 'needs_confirmation'
    : target.kind === 'monitor' && !monitors.some((monitor) => monitor.device_path === target.device_path)
      ? 'disconnected' : 'ready';
  const selected = monitors.filter((monitor) => target.kind === 'all' || target.device_path === monitor.device_path);
  const scope_hdr_state = target_status !== 'ready' || selected.length === 0 ? 'unknown'
    : selected.every((monitor) => monitor.is_hdr_enabled) ? 'hdr'
    : selected.some((monitor) => monitor.is_hdr_enabled) ? 'mixed' : 'sdr';
  return {
    is_hdr_active: scope_hdr_state === 'hdr', scope_hdr_state,
    current_app_name: null, current_exe: null, switched_by_app: false,
    steam_id: null, launcher: null, hdr_type: null, target_status,
    warning: target_status === 'disconnected' ? 'The saved display is disconnected. No other display is substituted.' : null,
    active_target: null, target_deferred: false, inventory_stale: false,
    uncertain_targets: [], operation_outcomes: [],
    any_hdr_active: monitors.some((monitor) => monitor.is_hdr_enabled),
  };
}

async function commit(changesLibrary = false, newHistory = false) {
  snapshot.control_epoch = String(BigInt(snapshot.control_epoch) + 1n);
  snapshot.revision = newHistory ? '1' : String(BigInt(snapshot.revision) + 1n);
  if (newHistory) {
    ++history;
    snapshot.store_id = `fixture-history-${history}`;
    snapshot.context_token = `fixture-context-${history}`;
    snapshot.mode = 'ready';
  }
  if (changesLibrary) snapshot.library_generation = String(BigInt(snapshot.library_generation) + 1n);
  await emit('config-changed', structuredClone(snapshot));
  await emit('hdr-status-changed', status());
  return structuredClone(snapshot);
}

mockIPC(async (command, args = {}) => {
  commands.push({ command, args: structuredClone(args) });
  if (args.expectedContext && args.expectedContext !== snapshot.context_token) throw new Error('Retired fixture history');
  switch (command) {
    case 'set_ui_language': return null;
    case 'get_config': return structuredClone(snapshot);
    case 'get_current_status': return status();
    case 'get_monitors': return monitors.map((monitor) => ({
      ...monitor,
      is_selected: snapshot.settings.target_monitor.kind === 'all'
        || snapshot.settings.target_monitor.device_path === monitor.device_path,
    }));
    case 'patch_settings':
      if (failSave) throw new Error('Simulated save failure; previous settings preserved');
      Object.assign(snapshot.settings, args.patch);
      return commit();
    case 'initialize_config':
    case 'reset_config':
      snapshot.settings = { ...settings, target_monitor: { kind: 'all' }, apps: [] };
      return commit(true, true);
    case 'restore_config':
    case 'import_legacy_config': return commit(true, true);
    case 'recheck_controller': return commit();
    case 'get_catalog': return games.map((game) => ({
      ...game, support_tier: 'native', notes: 'Nativní HDR podpora (PCGamingWiki)',
    }));
    case 'scan_installed_games': return {
      context_token: snapshot.context_token, library_generation: snapshot.library_generation,
      games: structuredClone(games),
    };
    case 'add_custom_app':
      snapshot.settings.apps.push(args.app);
      return commit(true);
    case 'import_detected_games':
      if (args.expectedLibraryGeneration !== snapshot.library_generation) throw new Error('Stale fixture scan');
      snapshot.settings.apps.push(...args.detected);
      return commit(true);
    case 'remove_app':
      snapshot.settings.apps = snapshot.settings.apps.filter((game) => game.exe_name !== args.exeName);
      return commit(true);
    case 'toggle_app':
      snapshot.settings.apps.find((game) => game.exe_name === args.exeName).enabled = args.enabled;
      return commit(true);
    case 'verify_game_paths': return {};
    case 'get_running_processes': return [];
    case 'pick_game_exe': return null;
    case 'sync_database': return games.length;
    case 'set_hdr': {
      const selected = monitors.filter((monitor) => args.scope.kind === 'all' || args.scope.device_path === monitor.device_path);
      const outcomes = selected.map((monitor) => {
        const previous = monitor.is_hdr_enabled;
        monitor.is_hdr_enabled = args.enable;
        return {
          device_path: monitor.device_path, display_name: monitor.name,
          requested_hdr: args.enable, outcome: previous === args.enable ? 'already_in_desired_state' : 'changed',
          failure: null, message: null, previous_hdr: previous, observed_hdr: args.enable, attempts: 1,
          previous_hdr_user_enabled: previous, observed_hdr_user_enabled: args.enable,
        };
      });
      await emit('hdr-status-changed', status());
      return { outcomes, partial: false, status: status() };
    }
    default: throw new Error(`Unexpected fixture command: ${command}`);
  }
}, { shouldMockEvents: true });

// The installed SDK mock expects `id`, while the real event API sends `eventId`.
const mockInvoke = window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke = (command, args, invokeOptions) => mockInvoke(
  command,
  command === 'plugin:event|unlisten' ? { ...args, id: args.eventId } : args,
  invokeOptions,
);

window.__hdrFixture = {
  commands,
  get snapshot() { return structuredClone(snapshot); },
  permitSaves() { failSave = false; },
  emitStatus: () => emit('hdr-status-changed', status()),
};
await import('../src/main.tsx');
