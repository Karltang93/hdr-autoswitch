export type HdrType = 'native' | 'autohdr' | 'media' | 'custom';

export type SupportTier =
  | 'native'
  | 'limited'
  | 'always_on'
  | 'manual_fix'
  | 'autohdr'
  | 'media'
  | 'custom';

export interface HdrApp {
  name: string;
  exe_name: string;
  enabled: boolean;
  hdr_type: HdrType;
  path?: string;
  alternate_exes?: string[];
  steam_id?: string;
  launcher?: string;
}

export interface PickedGameInfo {
  name: string;
  exe_name: string;
  path: string;
  hdr_type: HdrType;
  is_hdr_supported: boolean;
  notes?: string;
  launcher?: string;
}

export interface ActivityLogEntry {
  id: string;
  timestamp: string;
  message: string;
  type: 'info' | 'hdr_on' | 'hdr_off' | 'game' | 'system';
}

export interface CatalogEntry {
  name: string;
  exe_name: string;
  hdr_type: HdrType;
  support_tier: SupportTier;
  notes?: string;
}

export interface MonitorInfo {
  id: string;
  name: string;
  adapter_id_low: number;
  adapter_id_high: number;
  target_id: number;
  is_hdr_supported: boolean;
  is_hdr_enabled: boolean;
  is_primary: boolean;
}

export type SwitchMethod = 'native' | 'shortcut';

export interface AppConfig {
  target_monitor: string;
  alt_tab_delay_seconds: number;
  notifications_enabled: boolean;
  autostart: boolean;
  start_minimized: boolean;
  auto_detect_new_games: boolean;
  auto_sync_database: boolean;
  last_sync_timestamp?: number;
  exit_only_hdr: boolean;
  switch_method: SwitchMethod;
  blacklist: string[];
  apps: HdrApp[];
}

export interface RunningProcessInfo {
  pid: number;
  name: string;
  exe_name: string;
  title: string;
  path: string;
}

export interface HdrStatePayload {
  is_hdr_active: boolean;
  current_app_name?: string | null;
  current_exe?: string | null;
  switched_by_app: boolean;
  steam_id?: string | null;
  launcher?: string | null;
  hdr_type?: string | null;
}

export interface RecentGameSession {
  exe: string;
  name: string;
  steam_id?: string;
  launcher?: string;
  hdr_type: 'native' | 'autohdr' | 'media' | 'custom' | string;
  hdr_tier_label?: string;
  last_switched_at: string;
  hook_status: 'active' | 'switched_on' | 'switched_off';
  hook_message: string;
}

