import React, { useState } from 'react';
import { MonitorInfo, HdrStatePayload, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { Monitor, Tv, Sparkles, ShieldCheck, Clock, Zap, Cpu, Activity, RefreshCw } from 'lucide-react';
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
    <div className="space-y-6 animate-fadeIn">
      {/* Hero Status Card with Geometric Studio Aesthetic & Corner Brackets */}
      <div
        className={`relative overflow-hidden rounded-2xl p-6 transition-all duration-300 border corner-brackets ${
          status.is_hdr_active
            ? isDark
              ? 'bg-gradient-to-r from-rose-950/35 via-purple-950/25 to-slate-900/50 border-rose-500/40 glass-glow-hdr'
              : 'bg-gradient-to-r from-rose-50/80 via-purple-50/60 to-white/70 border-rose-300 shadow-md'
            : isDark
            ? 'bg-[#0b0f19]/70 border-white/[0.08] hover:border-white/[0.15]'
            : 'bg-white/80 border-slate-200 shadow-sm'
        } glass-panel`}
      >
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 relative z-10">
          <div className="flex items-start gap-4">
            <div
              className={`p-3.5 rounded-2xl transition-transform duration-500 border ${
                status.is_hdr_active
                  ? 'bg-[#0f1422] border-rose-500/40 shadow-[0_0_25px_rgba(244,63,94,0.3)] scale-105'
                  : isDark
                  ? 'bg-slate-900/80 border-white/10'
                  : 'bg-slate-100 border-slate-300'
              }`}
            >
              <HdrLogo size={46} active={status.is_hdr_active} />
            </div>

            <div>
              <div className="flex items-center gap-2 flex-wrap">
                <span
                  className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-md text-[11px] font-mono font-bold tracking-wider uppercase ${
                    status.is_hdr_active
                      ? 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
                      : isDark
                      ? 'bg-slate-800 text-slate-400 border border-white/5'
                      : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      status.is_hdr_active
                        ? 'bg-rose-400 animate-ping'
                        : 'bg-slate-500'
                    }`}
                  />
                  {status.is_hdr_active ? 'HDR10 // REC.2020 ACTIVE' : 'SDR // BT.709 STANDBY'}
                </span>

                {status.switched_by_app && (
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    AUTO-DETECTED
                  </span>
                )}

                <span className="text-[10px] font-mono text-slate-400">
                  MONITORS // {hdrSupportedMonitors.length} HDR CAPABLE
                </span>
              </div>

              <h2 className="text-xl md:text-2xl font-black mt-1.5 tracking-tight">
                {status.is_hdr_active
                  ? 'Windows High Dynamic Range je aktivní'
                  : 'Windows běží ve standardním SDR režimu'}
              </h2>

              <p className={`text-xs md:text-sm mt-1 font-medium ${isDark ? 'text-slate-400' : 'text-slate-600'}`}>
                {status.current_app_name ? (
                  <span className="flex items-center gap-1.5 text-rose-400">
                    <Sparkles className="w-3.5 h-3.5 inline text-amber-400" />
                    <span>Aktivní HDR proces: </span>
                    <strong className="text-white">{status.current_app_name}</strong>
                    {status.current_exe && (
                      <span className="text-xs opacity-75 font-mono text-slate-300">
                        [{status.current_exe}]
                      </span>
                    )}
                  </span>
                ) : (
                  <span className="flex items-center gap-1.5">
                    <Activity className="w-3.5 h-3.5 text-emerald-400" />
                    WinEventHook sleduje aktivní okna — HDR se zapne automaticky při vstupu do hry.
                  </span>
                )}
              </p>
            </div>
          </div>

          {/* Quick manual toggle button with electric neon styling */}
          <div className="flex items-center gap-3">
            <button
              onClick={handleToggle}
              disabled={toggling}
              className={`px-5 py-3 rounded-xl font-bold text-xs uppercase tracking-wider transition-all duration-300 flex items-center gap-2 cursor-pointer shadow-lg ${
                status.is_hdr_active
                  ? 'bg-gradient-to-r from-rose-600 to-rose-700 hover:from-rose-500 hover:to-rose-600 text-white shadow-rose-600/30 border border-rose-400/30'
                  : 'bg-gradient-to-r from-cyan-600 via-blue-600 to-indigo-600 hover:from-cyan-500 hover:to-blue-500 text-white shadow-cyan-600/25 border border-cyan-400/30'
              } disabled:opacity-50 hover:scale-[1.02] active:scale-[0.98]`}
            >
              <Zap className="w-4 h-4" />
              {toggling
                ? 'PŘEPÍNÁM...'
                : status.is_hdr_active
                ? 'VYPNOUT HDR'
                : 'ZAPNOUT HDR RUČNĚ'}
            </button>
          </div>
        </div>
      </div>

      {/* Connected Displays Section */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-4 h-4 text-cyan-400" />
            <h3 className="font-bold text-sm tracking-wide uppercase text-slate-200">
              Připojené monitory
            </h3>
            <span className="text-[10px] font-mono text-slate-500">
              [{monitors.length} DISPL.]
            </span>
          </div>
          <button
            onClick={onRefreshMonitors}
            className={`flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg border transition-colors cursor-pointer ${
              isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-300'
                : 'border-slate-300 hover:bg-slate-100 text-slate-700'
            }`}
          >
            <RefreshCw className="w-3 h-3 text-slate-400" />
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
                className={`p-4 rounded-xl border transition-all glass-panel corner-brackets ${
                  m.is_hdr_enabled
                    ? isDark
                      ? 'bg-slate-900/80 border-rose-500/40 shadow-[0_0_20px_rgba(244,63,94,0.12)]'
                      : 'bg-white/80 border-rose-300 shadow-sm'
                    : isDark
                    ? 'bg-[#090d16]/70 border-white/[0.06] hover:border-white/[0.12]'
                    : 'bg-white/60 border-slate-200'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="space-y-1.5">
                    <div className="flex items-center gap-2">
                      <h4 className="font-semibold text-sm truncate max-w-[210px]" title={m.name}>
                        {m.name}
                      </h4>
                      {m.is_primary && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300 font-bold">
                          PRIMARY
                        </span>
                      )}
                      {isTarget && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300 font-bold">
                          TARGET
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs">
                      {m.is_hdr_supported ? (
                        <span className="text-emerald-400 flex items-center gap-1 font-mono text-[11px]">
                          <ShieldCheck className="w-3.5 h-3.5" />
                          HDR10 SUPPORTED
                        </span>
                      ) : (
                        <span className="text-slate-500 font-mono text-[11px]">SDR ONLY</span>
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
                        className={`text-xs font-mono font-bold px-3 py-1.5 rounded-lg border transition-all cursor-pointer ${
                          m.is_hdr_enabled
                            ? 'bg-rose-500/20 text-rose-300 border-rose-500/40 hover:bg-rose-500/30'
                            : isDark
                            ? 'bg-slate-800 text-slate-300 border-white/10 hover:bg-slate-700'
                            : 'bg-slate-200 text-slate-800 border-slate-300 hover:bg-slate-300'
                        }`}
                      >
                        {m.is_hdr_enabled ? 'HDR ON' : 'SDR'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* System Metrics & Telemetry Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3.5">
        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#090d16]/70 border-white/[0.06] hover:border-cyan-500/30' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-mono mb-1">
            <Cpu className="w-3.5 h-3.5 text-cyan-400" />
            <span>CPU OVERHEAD</span>
          </div>
          <div className="text-lg font-black text-emerald-400 font-mono">0.0 %</div>
          <p className="text-[10px] text-slate-500 font-mono mt-0.5">WinEventHook zero loop</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#090d16]/70 border-white/[0.06] hover:border-purple-500/30' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-mono mb-1">
            <Sparkles className="w-3.5 h-3.5 text-purple-400" />
            <span>MONITORED GAMES</span>
          </div>
          <div className="text-lg font-black text-slate-100 font-mono">{activeAppsCount}</div>
          <p className="text-[10px] text-slate-500 font-mono mt-0.5">Aktivní v konfiguraci</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#090d16]/70 border-white/[0.06] hover:border-amber-500/30' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-mono mb-1">
            <Clock className="w-3.5 h-3.5 text-amber-400" />
            <span>ALT+TAB DEBOUNCE</span>
          </div>
          <div className="text-lg font-black text-slate-100 font-mono">{config.alt_tab_delay_seconds}s</div>
          <p className="text-[10px] text-slate-500 font-mono mt-0.5">Antiflicker prodleva</p>
        </div>

        <div
          className={`p-3.5 rounded-xl border glass-panel transition-all ${
            isDark ? 'bg-[#090d16]/70 border-white/[0.06] hover:border-rose-500/30' : 'bg-white/60 border-slate-200'
          }`}
        >
          <div className="flex items-center gap-2 text-slate-400 text-xs font-mono mb-1">
            <Tv className="w-3.5 h-3.5 text-rose-400" />
            <span>HDR DISPLAYS</span>
          </div>
          <div className="text-lg font-black text-slate-100 font-mono">
            {hdrSupportedMonitors.length} <span className="text-xs text-slate-500 font-normal">/ {monitors.length}</span>
          </div>
          <p className="text-[10px] text-slate-500 font-mono mt-0.5">Hardware kompatibilní</p>
        </div>
      </div>
    </div>
  );
};
