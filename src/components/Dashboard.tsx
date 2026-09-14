import React, { useState } from 'react';
import { MonitorInfo, HdrStatePayload, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Monitor,
  Sparkles,
  ShieldCheck,
  Clock,
  Zap,
  Cpu,
  RefreshCw,
  Tv,
} from 'lucide-react';
import { HdrLogo } from './HdrLogo';

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
    <div className="space-y-6">
      {/* Hero Display Control Center */}
      <div
        className={`relative overflow-hidden rounded-2xl p-6 transition-all duration-300 border glass-panel ${
          status.is_hdr_active
            ? isDark
              ? 'bg-gradient-to-r from-rose-950/25 via-purple-950/20 to-[#0c0f18]/80 border-rose-500/30 glass-glow-hdr'
              : 'bg-gradient-to-r from-rose-50/70 via-purple-50/50 to-white/80 border-rose-200 shadow-md'
            : isDark
            ? 'bg-[#0c0f18]/80 border-white/[0.07] hover:border-white/[0.12]'
            : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 relative z-10">
          <div className="flex items-start gap-4">
            {/* Logo Container with Ambient Glow */}
            <div
              className={`p-3.5 rounded-2xl border transition-all duration-300 shrink-0 ${
                status.is_hdr_active
                  ? 'bg-[#121524] border-rose-500/30 shadow-[0_0_25px_rgba(244,63,94,0.25)]'
                  : isDark
                  ? 'bg-white/[0.03] border-white/[0.08]'
                  : 'bg-slate-100 border-slate-200'
              }`}
            >
              <HdrLogo size={48} active={status.is_hdr_active} />
            </div>

            <div className="space-y-1">
              <div className="flex items-center gap-2 flex-wrap">
                <span
                  className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium transition-all ${
                    status.is_hdr_active
                      ? 'bg-rose-500/15 text-rose-300 border border-rose-500/30'
                      : isDark
                      ? 'bg-white/[0.04] text-slate-400 border border-white/[0.07]'
                      : 'bg-slate-100 text-slate-600 border border-slate-200'
                  }`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      status.is_hdr_active ? 'bg-rose-400 animate-pulse' : 'bg-slate-400'
                    }`}
                  />
                  {status.is_hdr_active ? 'HDR10 Aktivní' : 'SDR Standby'}
                </span>

                {status.switched_by_app && (
                  <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                    Automaticky detekováno
                  </span>
                )}

                <span className="text-[11px] text-slate-400">
                  {hdrSupportedMonitors.length} HDR {hdrSupportedMonitors.length === 1 ? 'displej' : 'displeje'}
                </span>
              </div>

              <h2 className="text-xl md:text-2xl font-bold tracking-tight mt-1">
                {status.is_hdr_active
                  ? 'Windows High Dynamic Range je aktivní'
                  : 'Windows běží ve standardním SDR režimu'}
              </h2>

              <p className={`text-xs md:text-sm font-normal ${isDark ? 'text-slate-400' : 'text-slate-600'}`}>
                {status.current_app_name ? (
                  <span className="flex items-center gap-1.5 text-rose-300">
                    <Sparkles className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                    <span>Aktivní HDR hra:</span>
                    <strong className="text-white font-semibold">{status.current_app_name}</strong>
                    {status.current_exe && (
                      <span className="text-xs opacity-75 font-mono text-slate-300">
                        ({status.current_exe})
                      </span>
                    )}
                  </span>
                ) : (
                  <span>WinEventHook sleduje okna — HDR se zapne automaticky při spuštění hry.</span>
                )}
              </p>
            </div>
          </div>

          {/* Quick Manual Toggle Button */}
          <div className="shrink-0 flex items-center">
            <button
              onClick={handleToggle}
              disabled={toggling}
              className={`px-5 py-2.5 rounded-xl font-semibold text-xs tracking-wide transition-all duration-200 flex items-center gap-2 cursor-pointer ${
                status.is_hdr_active
                  ? 'bg-rose-500/20 hover:bg-rose-500/30 text-rose-200 border border-rose-500/40 hover:border-rose-400/60 shadow-sm'
                  : 'bg-sky-500 hover:bg-sky-400 text-slate-950 font-bold shadow-md hover:shadow-sky-500/20 active:scale-95'
              } disabled:opacity-50`}
            >
              <Zap className="w-4 h-4" />
              <span>
                {toggling
                  ? 'Přepínám...'
                  : status.is_hdr_active
                  ? 'Vypnout HDR'
                  : 'Zapnout HDR ručně'}
              </span>
            </button>
          </div>
        </div>
      </div>

      {/* Connected Displays Section */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-4 h-4 text-sky-400" />
            <h3 className="font-semibold text-sm text-slate-200">
              Připojené monitory
            </h3>
            <span className="text-xs text-slate-500 font-mono">
              ({monitors.length})
            </span>
          </div>

          <button
            onClick={onRefreshMonitors}
            className={`flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-lg border transition-colors cursor-pointer ${
              isDark
                ? 'border-white/[0.08] hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700'
            }`}
          >
            <RefreshCw className="w-3 h-3 text-slate-400" />
            <span>Obnovit</span>
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
          {monitors.map((m) => {
            const isTarget =
              config.target_monitor === 'all' || config.target_monitor === m.id;

            return (
              <div
                key={m.id}
                className={`p-4 rounded-xl border transition-all glass-panel ${
                  m.is_hdr_enabled
                    ? isDark
                      ? 'bg-rose-950/20 border-rose-500/30'
                      : 'bg-white/80 border-rose-200 shadow-sm'
                    : isDark
                    ? 'bg-[#0c0f18]/70 border-white/[0.06] hover:border-white/[0.12]'
                    : 'bg-white/70 border-slate-200'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <h4 className="font-semibold text-sm truncate max-w-[220px]" title={m.name}>
                        {m.name}
                      </h4>
                      {m.is_primary && (
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-sky-500/15 text-sky-300 font-medium">
                          Primární
                        </span>
                      )}
                      {isTarget && (
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-purple-500/15 text-purple-300 font-medium">
                          Cíl
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs">
                      {m.is_hdr_supported ? (
                        <span className="text-emerald-400 flex items-center gap-1 text-[11px] font-medium">
                          <ShieldCheck className="w-3.5 h-3.5" />
                          HDR10 Podporováno
                        </span>
                      ) : (
                        <span className="text-slate-500 text-[11px]">Pouze SDR</span>
                      )}
                      <span className="text-slate-600">•</span>
                      <span className="text-slate-400 font-mono text-[11px]">
                        ID: {m.target_id}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {m.is_hdr_supported && (
                      <button
                        onClick={() => handleToggleMonitor(m)}
                        className={`text-xs font-semibold px-3 py-1 rounded-lg border transition-all cursor-pointer ${
                          m.is_hdr_enabled
                            ? 'bg-rose-500/20 text-rose-300 border-rose-500/30 hover:bg-rose-500/30'
                            : isDark
                            ? 'bg-white/[0.04] text-slate-300 border-white/[0.08] hover:bg-white/[0.08]'
                            : 'bg-slate-100 text-slate-800 border-slate-200 hover:bg-slate-200'
                        }`}
                      >
                        {m.is_hdr_enabled ? 'HDR Zapnuto' : 'SDR'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Telemetry Metrics Row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3.5">
        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#0c0f18]/70 border-white/[0.06]' : 'bg-white/70 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-1.5 text-slate-400 text-xs mb-1">
            <Cpu className="w-3.5 h-3.5 text-sky-400" />
            <span>Využití CPU</span>
          </div>
          <div className="text-lg font-bold text-emerald-400 font-mono">0.0 %</div>
          <p className="text-[11px] text-slate-500 mt-0.5">WinEventHook bez smyčky</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#0c0f18]/70 border-white/[0.06]' : 'bg-white/70 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-1.5 text-slate-400 text-xs mb-1">
            <Sparkles className="w-3.5 h-3.5 text-purple-400" />
            <span>Sledované hry</span>
          </div>
          <div className="text-lg font-bold text-slate-100 font-mono">{activeAppsCount}</div>
          <p className="text-[11px] text-slate-500 mt-0.5">Aktivní v seznamu</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#0c0f18]/70 border-white/[0.06]' : 'bg-white/70 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-1.5 text-slate-400 text-xs mb-1">
            <Clock className="w-3.5 h-3.5 text-amber-400" />
            <span>Alt+Tab prodleva</span>
          </div>
          <div className="text-lg font-bold text-slate-100 font-mono">{config.alt_tab_delay_seconds}s</div>
          <p className="text-[11px] text-slate-500 mt-0.5">Ochrana proti probliknutí</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#0c0f18]/70 border-white/[0.06]' : 'bg-white/70 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-1.5 text-slate-400 text-xs mb-1">
            <Tv className="w-3.5 h-3.5 text-rose-400" />
            <span>HDR Monitory</span>
          </div>
          <div className="text-lg font-bold text-slate-100 font-mono">
            {hdrSupportedMonitors.length} <span className="text-xs text-slate-500 font-normal">/ {monitors.length}</span>
          </div>
          <p className="text-[11px] text-slate-500 mt-0.5">Kompatibilní hardware</p>
        </div>
      </div>
    </div>
  );
};
