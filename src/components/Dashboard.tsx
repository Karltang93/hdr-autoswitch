import React, { useState } from 'react';
import { MonitorInfo, HdrStatePayload, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { Monitor, Tv, Flame, Sparkles, ShieldCheck, Clock, Zap, Cpu } from 'lucide-react';

interface DashboardProps {
  status: HdrStatePayload;
  monitors: MonitorInfo[];
  config: AppConfig;
  onRefreshMonitors: () => void;
  onManualToggle: (enable: boolean) => void;
  isDark: boolean;
}

export const Dashboard: React.FC<DashboardProps> = ({
  status,
  monitors,
  config,
  onRefreshMonitors,
  onManualToggle,
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

  const activeAppsCount = config.apps.filter((a) => a.enabled).length;
  const hdrSupportedMonitors = monitors.filter((m) => m.is_hdr_supported);

  return (
    <div className="space-y-6 animate-fadeIn">
      {/* Hero Status Card with Frosted Glass & Neon Glow */}
      <div
        className={`relative overflow-hidden rounded-2xl p-6 transition-all duration-300 border ${
          status.is_hdr_active
            ? isDark
              ? 'bg-gradient-to-r from-rose-950/40 via-purple-950/30 to-amber-950/20 border-rose-500/30 glass-glow-hdr'
              : 'bg-gradient-to-r from-rose-100/80 via-purple-50/70 to-amber-50/60 border-rose-300/60 shadow-lg'
            : isDark
            ? 'bg-slate-900/60 border-white/10'
            : 'bg-white/70 border-slate-200 shadow-sm'
        } glass-panel`}
      >
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
          <div className="flex items-start gap-4">
            <div
              className={`p-4 rounded-2xl transition-transform duration-300 ${
                status.is_hdr_active
                  ? 'bg-gradient-to-br from-amber-500 via-rose-500 to-purple-600 text-white shadow-lg shadow-rose-500/30 scale-105'
                  : isDark
                  ? 'bg-slate-800 text-slate-400'
                  : 'bg-slate-200 text-slate-600'
              }`}
            >
              {status.is_hdr_active ? (
                <Flame className="w-8 h-8 animate-pulse" />
              ) : (
                <Tv className="w-8 h-8" />
              )}
            </div>

            <div>
              <div className="flex items-center gap-2">
                <span
                  className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold ${
                    status.is_hdr_active
                      ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                      : isDark
                      ? 'bg-slate-800 text-slate-400'
                      : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full mr-1.5 ${
                      status.is_hdr_active
                        ? 'bg-rose-500 animate-ping'
                        : 'bg-slate-500'
                    }`}
                  />
                  {status.is_hdr_active ? 'HDR ZAPNUTO' : 'SDR REŽIM'}
                </span>

                {status.switched_by_app && (
                  <span className="text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    Auto-detekce
                  </span>
                )}
              </div>

              <h2 className="text-2xl font-bold mt-1 tracking-tight">
                {status.is_hdr_active
                  ? 'Windows High Dynamic Range je aktivní'
                  : 'Windows běží ve standardním SDR'}
              </h2>

              <p className={`text-sm mt-1 ${isDark ? 'text-slate-400' : 'text-slate-600'}`}>
                {status.current_app_name ? (
                  <span className="flex items-center gap-1.5 text-rose-400 font-medium">
                    <Sparkles className="w-4 h-4 inline" /> Hrajete:{' '}
                    <strong>{status.current_app_name}</strong>
                    {status.current_exe && (
                      <span className="text-xs opacity-75 font-mono">
                        ({status.current_exe})
                      </span>
                    )}
                  </span>
                ) : (
                  'Čeká na spuštění nebo přepnutí do HDR hry / přehrávače médií.'
                )}
              </p>
            </div>
          </div>

          {/* Quick manual toggle button */}
          <div className="flex items-center gap-3">
            <button
              onClick={handleToggle}
              disabled={toggling}
              className={`px-5 py-3 rounded-xl font-semibold text-sm transition-all duration-200 flex items-center gap-2 cursor-pointer shadow-md ${
                status.is_hdr_active
                  ? 'bg-rose-600 hover:bg-rose-700 text-white shadow-rose-600/20'
                  : 'bg-cyan-600 hover:bg-cyan-700 text-white shadow-cyan-600/20'
              } disabled:opacity-50`}
            >
              <Zap className="w-4 h-4" />
              {toggling
                ? 'Přepínám...'
                : status.is_hdr_active
                ? 'Vypnout HDR'
                : 'Zapnout HDR ručně'}
            </button>
          </div>
        </div>
      </div>

      {/* Connected Displays */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-5 h-5 text-cyan-500" />
            <h3 className="font-semibold text-lg">Připojené monitory</h3>
          </div>
          <button
            onClick={onRefreshMonitors}
            className={`text-xs px-2.5 py-1 rounded-lg border transition-colors cursor-pointer ${
              isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-300'
                : 'border-slate-300 hover:bg-slate-100 text-slate-700'
            }`}
          >
            Obnovit
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {monitors.map((m) => {
            const isTarget =
              config.target_monitor === 'all' || config.target_monitor === m.id;

            return (
              <div
                key={m.id}
                className={`p-4 rounded-xl border transition-all glass-panel ${
                  m.is_hdr_enabled
                    ? isDark
                      ? 'bg-slate-900/70 border-rose-500/40 shadow-sm'
                      : 'bg-white/80 border-rose-300 shadow-sm'
                    : isDark
                    ? 'bg-slate-900/40 border-white/5'
                    : 'bg-white/60 border-slate-200'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <h4 className="font-medium text-sm truncate max-w-[200px]" title={m.name}>
                        {m.name}
                      </h4>
                      {m.is_primary && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-400 font-semibold">
                          Hlavní
                        </span>
                      )}
                      {isTarget && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-400 font-semibold">
                          Cílový
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs">
                      {m.is_hdr_supported ? (
                        <span className="text-emerald-400 flex items-center gap-1">
                          <ShieldCheck className="w-3.5 h-3.5" /> Podpora HDR
                        </span>
                      ) : (
                        <span className="text-slate-500">Pouze SDR</span>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {m.is_hdr_supported && (
                      <button
                        onClick={() => handleToggleMonitor(m)}
                        className={`text-xs font-medium px-3 py-1.5 rounded-lg border transition-all cursor-pointer ${
                          m.is_hdr_enabled
                            ? 'bg-rose-500/20 text-rose-300 border-rose-500/30 hover:bg-rose-500/30'
                            : isDark
                            ? 'bg-slate-800 text-slate-300 border-white/10 hover:bg-slate-700'
                            : 'bg-slate-200 text-slate-800 border-slate-300 hover:bg-slate-300'
                        }`}
                      >
                        {m.is_hdr_enabled ? 'HDR Zap' : 'SDR'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* System Metrics & Info Bar */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div
          className={`p-4 rounded-xl border glass-panel ${
            isDark ? 'bg-slate-900/40 border-white/5' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-medium mb-1">
            <Cpu className="w-4 h-4 text-cyan-400" />
            <span>Zátěž procesoru</span>
          </div>
          <div className="text-xl font-bold text-emerald-400">0.0 %</div>
          <p className="text-[11px] text-slate-500 mt-0.5">WinEventHook bez cyklů</p>
        </div>

        <div
          className={`p-4 rounded-xl border glass-panel ${
            isDark ? 'bg-slate-900/40 border-white/5' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-medium mb-1">
            <Sparkles className="w-4 h-4 text-purple-400" />
            <span>Sledované hry</span>
          </div>
          <div className="text-xl font-bold">{activeAppsCount}</div>
          <p className="text-[11px] text-slate-500 mt-0.5">V databázi aktivní</p>
        </div>

        <div
          className={`p-4 rounded-xl border glass-panel ${
            isDark ? 'bg-slate-900/40 border-white/5' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-medium mb-1">
            <Clock className="w-4 h-4 text-amber-400" />
            <span>Alt+Tab zpoždění</span>
          </div>
          <div className="text-xl font-bold">{config.alt_tab_delay_seconds} s</div>
          <p className="text-[11px] text-slate-500 mt-0.5">Ochrana proti probliku</p>
        </div>

        <div
          className={`p-4 rounded-xl border glass-panel ${
            isDark ? 'bg-slate-900/40 border-white/5' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-medium mb-1">
            <Tv className="w-4 h-4 text-rose-400" />
            <span>HDR Monitory</span>
          </div>
          <div className="text-xl font-bold">{hdrSupportedMonitors.length}</div>
          <p className="text-[11px] text-slate-500 mt-0.5">
            Z {monitors.length} připojených
          </p>
        </div>
      </div>
    </div>
  );
};
