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
}
