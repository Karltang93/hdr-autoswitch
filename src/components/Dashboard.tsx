import React, { useState } from 'react';
import { MonitorInfo, HdrStatePayload, AppConfig, ActivityLogEntry } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Monitor,
  Sparkles,
  ShieldCheck,
  Zap,
  RefreshCw,
  Gamepad2,
  Terminal,
  ArrowRight,
  CheckCircle2,
} from 'lucide-react';
import { HdrLogo } from './HdrLogo';

interface DashboardProps {
  status: HdrStatePayload;
  monitors: MonitorInfo[];
  config: AppConfig;
  activityLogs: ActivityLogEntry[];
  onRefreshMonitors: () => void;
  onManualToggle: (enable: boolean) => void;
  onNavigateToApps: () => void;
  onUpdateConfig: (newConfig: AppConfig) => void;
  isDark: boolean;
}

export const Dashboard: React.FC<DashboardProps> = ({
  status,
  monitors,
  config,
  activityLogs,
  onRefreshMonitors,
  onManualToggle,
  onNavigateToApps,
  onUpdateConfig,
  isDark,
}) => {
  const [toggling, setToggling] = useState(false);

  const handleToggle = async () => {
    setToggling(true);
    try {
      const nextState = !status.is_hdr_active;
      await invoke('toggle_all_hdr', { enable: nextState });
      onManualToggle(nextState);
      setTimeout(onRefreshMonitors, 500);
    } catch (err) {
      console.error('Failed to toggle HDR:', err);
    } finally {
      setToggling(false);
    }
  };

  const handleToggleMonitor = async (m: MonitorInfo) => {
    if (!m.is_hdr_supported) return;
    try {
      await invoke('set_monitor_hdr', {
        adapterLow: m.adapter_id_low,
        adapterHigh: m.adapter_id_high,
        targetId: m.target_id,
        enable: !m.is_hdr_enabled,
      });
      setTimeout(onRefreshMonitors, 600);
    } catch (err) {
      console.error('Failed to toggle monitor HDR:', err);
    }
  };

  const handleToggleApp = async (exeName: string, enabled: boolean) => {
    try {
      await invoke('toggle_app', { exeName, enabled });
      const updatedApps = config.apps.map((a) =>
        a.exe_name.toLowerCase() === exeName.toLowerCase() ? { ...a, enabled } : a
      );
      onUpdateConfig({ ...config, apps: updatedApps });
    } catch (err) {
      console.error('Failed to toggle app:', err);
    }
  };

  const hdrSupportedMonitors = monitors.filter((m) => m.is_hdr_supported);
  const recentApps = config.apps.slice(0, 6);

  return (
    <div className="space-y-6">
      {/* Hero Display Control Center */}
      <div
        className={`relative overflow-hidden rounded-3xl p-7 transition-all duration-300 border glass-panel ${
          status.is_hdr_active
            ? isDark
              ? 'bg-gradient-to-r from-rose-950/30 via-purple-950/25 to-[#0e1322]/80 border-rose-500/40 neon-glow-rose'
              : 'bg-gradient-to-r from-rose-50/90 via-purple-50/70 to-white/90 border-rose-300 shadow-lg'
            : isDark
            ? 'bg-[#0f1422]/70 border-white/[0.08] hover:border-cyan-500/30'
            : 'bg-white/90 border-slate-200/90 shadow-md'
        }`}
      >
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6 relative z-10">
          <div className="flex items-center gap-5">
            {/* Ambient Glowing Aperture Dial */}
            <div
              className={`p-4 rounded-2xl border transition-all duration-300 shrink-0 ${
                status.is_hdr_active
                  ? 'bg-[#14192b] border-rose-500/40 shadow-[0_0_30px_rgba(244,63,94,0.35)] scale-105'
                  : isDark
                  ? 'bg-white/[0.04] border-white/[0.08] hover:border-cyan-500/40'
                  : 'bg-slate-100 border-slate-200'
              }`}
            >
              <HdrLogo size={56} active={status.is_hdr_active} />
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-2.5 flex-wrap">
                <span
                  className={`inline-flex items-center gap-1.5 px-3 py-0.5 rounded-full text-xs font-bold tracking-wider uppercase transition-all ${
                    status.is_hdr_active
                      ? 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
                      : isDark
                      ? 'bg-cyan-950/40 text-cyan-400 border border-cyan-500/20'
                      : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  <span
                    className={`w-2 h-2 rounded-full ${
                      status.is_hdr_active ? 'bg-rose-400 animate-status-pulse' : 'bg-cyan-400'
                    }`}
                  />
                  {status.is_hdr_active ? 'HDR10 REC.2020 AKTIVNÍ' : 'SDR BT.709 STANDBY'}
                </span>

                {status.switched_by_app && (
                  <span className="text-xs px-2.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 font-medium">
                    Automaticky detekováno
                  </span>
                )}

                <span className="text-xs text-slate-400 font-medium">
                  {hdrSupportedMonitors.length} HDR {hdrSupportedMonitors.length === 1 ? 'displej' : 'displeje'} připraveno
                </span>
              </div>

              <h2 className="text-2xl lg:text-3xl font-extrabold tracking-tight">
                {status.is_hdr_active
                  ? 'Windows High Dynamic Range je aktivní'
                  : 'Windows běží ve standardním SDR režimu'}
              </h2>

              <p className={`text-sm ${isDark ? 'text-slate-400' : 'text-slate-600'}`}>
                {status.current_app_name ? (
                  <span className="flex items-center gap-2 text-rose-300 font-medium">
                    <Sparkles className="w-4 h-4 text-amber-400 shrink-0" />
                    <span>Aktivní HDR proces:</span>
                    <strong className="text-white text-base font-bold">{status.current_app_name}</strong>
                    {status.current_exe && (
                      <span className="text-xs opacity-75 font-mono text-slate-300">
                        [{status.current_exe}]
                      </span>
                    )}
                  </span>
                ) : (
                  <span>WinEventHook sleduje okna — jakmile spustíte HDR hru, displej se bleskově přepne.</span>
                )}
              </p>
            </div>
          </div>

          {/* Large tactile glowing toggle button */}
          <div className="shrink-0 flex items-center">
            <button
              onClick={handleToggle}
              disabled={toggling}
              className={`px-6 py-3.5 rounded-2xl font-bold text-sm tracking-wide transition-all duration-200 flex items-center gap-3 cursor-pointer shadow-lg active:scale-95 ${
                status.is_hdr_active
                  ? 'bg-gradient-to-r from-rose-600 to-rose-700 hover:from-rose-500 hover:to-rose-600 text-white shadow-rose-600/30 border border-rose-400/40 neon-glow-rose'
                  : 'bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white shadow-cyan-500/25 border border-cyan-400/30 neon-glow-cyan'
              } disabled:opacity-50`}
            >
              <Zap className="w-5 h-5 fill-current" />
              <span>
                {toggling
                  ? 'Přepínám displeje...'
                  : status.is_hdr_active
                  ? 'Vypnout HDR'
                  : 'Zapnout HDR ručně'}
              </span>
            </button>
          </div>
        </div>
      </div>

      {/* Connected Displays & Monitor Controls */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-4 h-4 text-cyan-400" />
            <h3 className="font-bold text-sm tracking-wide uppercase text-slate-200">
              Připojené displeje
            </h3>
            <span className="text-xs text-slate-500 font-mono">
              ({monitors.length})
            </span>
          </div>

          <button
            onClick={onRefreshMonitors}
            className={`flex items-center gap-1.5 text-xs font-semibold px-3 py-1.5 rounded-xl border transition-all cursor-pointer ${
              isDark
                ? 'border-white/[0.08] hover:border-cyan-500/40 hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-xs'
            }`}
          >
            <RefreshCw className="w-3.5 h-3.5 text-cyan-400" />
            <span>Obnovit</span>
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {monitors.map((m) => {
            const isTarget =
              config.target_monitor === 'all' || config.target_monitor === m.id;

            return (
              <div
                key={m.id}
                className={`p-4 rounded-2xl border transition-all glass-panel ${
                  m.is_hdr_enabled
                    ? isDark
                      ? 'bg-rose-950/20 border-rose-500/40 neon-glow-rose'
                      : 'bg-white/90 border-rose-300 shadow-sm'
                    : isDark
                    ? 'bg-[#0f1422]/60 border-white/[0.06] hover:border-cyan-500/30'
                    : 'bg-white/80 border-slate-200'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4 className="font-bold text-sm truncate max-w-[240px]" title={m.name}>
                        {m.name}
                      </h4>
                      {m.is_primary && (
                        <span className="text-[10px] px-2 py-0.5 rounded-md bg-cyan-500/15 text-cyan-300 font-bold">
                          Primární
                        </span>
                      )}
                      {isTarget && (
                        <span className="text-[10px] px-2 py-0.5 rounded-md bg-purple-500/15 text-purple-300 font-bold">
                          Cíl HDR
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs">
                      {m.is_hdr_supported ? (
                        <span className="text-emerald-400 flex items-center gap-1 text-xs font-semibold">
                          <ShieldCheck className="w-4 h-4 text-emerald-400" />
                          HDR10 Podporováno
                        </span>
                      ) : (
                        <span className="text-slate-500 text-xs">Pouze SDR</span>
                      )}
                      <span className="text-slate-600">•</span>
                      <span className="text-slate-400 font-mono text-xs">
                        Target ID: {m.target_id}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {m.is_hdr_supported && (
                      <button
                        onClick={() => handleToggleMonitor(m)}
                        className={`text-xs font-bold px-3.5 py-1.5 rounded-xl border transition-all cursor-pointer ${
                          m.is_hdr_enabled
                            ? 'bg-rose-500/20 text-rose-300 border-rose-500/40 hover:bg-rose-500/30'
                            : isDark
                            ? 'bg-white/[0.05] text-slate-300 border-white/10 hover:bg-white/10 hover:border-cyan-500/30'
                            : 'bg-slate-200 text-slate-800 border-slate-300 hover:bg-slate-300'
                        }`}
                      >
                        {m.is_hdr_enabled ? 'HDR ZAPNUTO' : 'SDR'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Quick Launch / Recent Games Section */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Gamepad2 className="w-4 h-4 text-purple-400" />
            <h3 className="font-bold text-sm tracking-wide uppercase text-slate-200">
              Sledované hry v rychlém přehledu
            </h3>
          </div>
          <button
            onClick={onNavigateToApps}
            className="text-xs text-cyan-400 hover:text-cyan-300 flex items-center gap-1 font-semibold cursor-pointer"
          >
            <span>Zobrazit všechny ({config.apps.length})</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
          {recentApps.map((app) => {
            const steamCover = app.steam_id
              ? `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${app.steam_id}/library_600x900.jpg`
              : null;

            return (
              <div
                key={app.exe_name}
                className={`relative rounded-2xl overflow-hidden border transition-all duration-200 hover-card-lift glass-panel flex flex-col justify-end ${
                  app.enabled
                    ? isDark
                      ? 'bg-[#101524]/80 border-white/[0.08] hover:border-cyan-500/50 hover:shadow-cyan-500/10'
                      : 'bg-white/90 border-slate-200 shadow-xs'
                    : 'opacity-60 bg-slate-900/40 border-white/[0.04]'
                }`}
                style={{ height: '140px' }}
              >
                {steamCover ? (
                  <img
                    src={steamCover}
                    alt={app.name}
                    className="absolute inset-0 w-full h-full object-cover"
                    onError={(e) => {
                      (e.target as HTMLElement).style.display = 'none';
                    }}
                  />
                ) : (
                  <div className="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-900 to-indigo-950" />
                )}

                {/* Dark Gradient Overlay */}
                <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/40 to-transparent pointer-events-none" />

                {/* Content Overlay */}
                <div className="relative z-10 p-2.5 space-y-1">
                  <div className="font-bold text-xs truncate text-white drop-shadow-md">
                    {app.name}
                  </div>
                  <div className="flex items-center justify-between">
                    <span
                      className={`text-[9px] font-mono px-1.5 py-0.2 rounded font-bold ${
                        app.enabled
                          ? 'bg-emerald-500/20 text-emerald-400'
                          : 'bg-slate-700 text-slate-400'
                      }`}
                    >
                      {app.enabled ? 'Sledováno' : 'Vypnuto'}
                    </span>

                    <button
                      onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                      className={`w-4 h-4 rounded-full flex items-center justify-center cursor-pointer transition-colors ${
                        app.enabled ? 'bg-cyan-500 text-black' : 'bg-slate-700 text-slate-400'
                      }`}
                      title={app.enabled ? 'Pozastavit' : 'Aktivovat'}
                    >
                      <CheckCircle2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Activity Log (Real-time System & Game Event Feed) */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-cyan-400" />
          <h3 className="font-bold text-sm tracking-wide uppercase text-slate-200">
            Záznam aktivity
          </h3>
        </div>

        <div
          className={`p-4 rounded-2xl border glass-panel space-y-2 max-h-[160px] overflow-y-auto ${
            isDark ? 'bg-[#0b0e18]/80 border-white/[0.06]' : 'bg-white/80 border-slate-200'
          }`}
        >
          {activityLogs.map((log) => (
            <div
              key={log.id}
              className="flex items-center gap-2.5 text-xs font-mono transition-colors hover:text-white"
            >
              <span className="text-slate-500 shrink-0">[{log.timestamp}]</span>
              <span
                className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                  log.type === 'hdr_on'
                    ? 'bg-rose-400'
                    : log.type === 'hdr_off'
                    ? 'bg-amber-400'
                    : log.type === 'game'
                    ? 'bg-cyan-400'
                    : 'bg-emerald-400'
                }`}
              />
              <span className="text-slate-300 truncate">{log.message}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
