import React, { useState } from 'react';
import { MonitorInfo, HdrStatePayload, AppConfig, ActivityLogEntry, RecentGameSession } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Monitor,
  ShieldCheck,
  RefreshCw,
  Terminal,
  ArrowRight,
  Sparkles,
  History,
  Zap,
} from 'lucide-react';
import { HdrLogo } from './HdrLogo';
import { GlitchButton } from './GlitchButton';
import { GlitchText } from './GlitchText';
import { useI18n } from '../i18n';

interface DashboardProps {
  status: HdrStatePayload;
  monitors: MonitorInfo[];
  config: AppConfig;
  activityLogs: ActivityLogEntry[];
  recentGames: RecentGameSession[];
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
  recentGames,
  onRefreshMonitors,
  onManualToggle,
  onNavigateToApps,
}) => {
  const { t } = useI18n();
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

  const hdrSupportedMonitors = monitors.filter((m) => m.is_hdr_supported);

  return (
    <div className="space-y-6 font-mono">
      {/* Hero Display Control Center (Retro Glitch Terminal) */}
      <div
        className={`relative overflow-hidden border p-6 transition-all duration-300 ${
          status.is_hdr_active
            ? 'bg-[#180e10] border-[#f55a6b] neon-glow-coral'
            : 'bg-[#120d0e] border-[#f55a6b]/30 hover:border-[#f55a6b]/60'
        }`}
      >
        {/* Subtle scanline background texture */}
        <div className="absolute inset-0 scanlines-overlay opacity-30 pointer-events-none" />

        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6 relative z-10">
          <div className="flex items-center gap-5">
            {/* Ambient Aperture Dial with Glitch Border */}
            <div
              className={`p-3.5 border shrink-0 transition-all duration-300 ${
                status.is_hdr_active
                  ? 'bg-[#221314] border-[#f55a6b] shadow-[0_0_25px_rgba(245,90,107,0.5)] scale-105'
                  : 'bg-[#170f10] border-[#f55a6b]/40'
              }`}
            >
              <HdrLogo size={52} active={status.is_hdr_active} />
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-2.5 flex-wrap">
                <span
                  className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 text-xs font-bold uppercase tracking-wider border ${
                    status.is_hdr_active
                      ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                      : 'bg-[#221314] text-[#5accf5] border-[#5accf5]/40'
                  }`}
                >
                  <span
                    className={`w-2 h-2 ${
                      status.is_hdr_active ? 'bg-[#0f0b0b] animate-status-pulse' : 'bg-[#5accf5]'
                    }`}
                  />
                  {status.is_hdr_active ? t.heroHdrRec2020 : t.heroSdrBt709}
                </span>

                {status.switched_by_app && (
                  <span className="text-xs px-2 py-0.5 bg-[#5accf5]/15 text-[#5accf5] border border-[#5accf5]/40">
                    {t.heroHookActive}
                  </span>
                )}

                <span className="text-xs text-[#8a7f81]">
                  {t.heroDisplaysReady(hdrSupportedMonitors.length)}
                </span>
              </div>

              <h2 className="text-2xl font-bold tracking-tight text-white flex items-center gap-2">
                <GlitchText
                  text={status.is_hdr_active ? t.heroHdrActiveTitle : t.heroSdrTitle}
                  scrambleOnHover={true}
                />
              </h2>

              <p className="text-xs text-[#b5a9ac]">
                {status.current_app_name ? (
                  <span className="flex items-center gap-2 text-[#f55a6b]">
                    <Sparkles className="w-4 h-4 text-[#5accf5] shrink-0" />
                    <span>{t.heroActiveProcess}</span>
                    <strong className="text-white font-bold tracking-wide">
                      {status.current_app_name}
                    </strong>
                    {status.current_exe && (
                      <span className="text-[#5accf5]">[{status.current_exe}]</span>
                    )}
                  </span>
                ) : (
                  <span>{t.heroSdrSubtext}</span>
                )}
              </p>
            </div>
          </div>

          {/* Large tactile glitch toggle button */}
          <div className="shrink-0 flex items-center">
            <GlitchButton
              label={
                toggling
                  ? t.heroSwitching
                  : status.is_hdr_active
                  ? t.heroTurnOffHdr
                  : t.heroTurnOnHdr
              }
              variant={status.is_hdr_active ? 'outline' : 'primary'}
              icon={<Zap className="w-4 h-4 fill-current" />}
              size="lg"
              disabled={toggling}
              onClick={handleToggle}
            />
          </div>
        </div>
      </div>

      {/* Connected Displays & Monitor Controls */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="w-4 h-4 text-[#5accf5]" />
            <h3 className="font-bold text-xs uppercase tracking-wider text-[#f55a6b]">
              {t.displaysTitle}
            </h3>
            <span className="text-xs text-[#8a7f81]">({monitors.length})</span>
          </div>

          <GlitchButton
            label={t.displaysRefresh}
            variant="outline"
            size="sm"
            icon={<RefreshCw className="w-3 h-3 text-[#5accf5]" />}
            onClick={onRefreshMonitors}
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {monitors.map((m) => {
            const isTarget =
              config.target_monitor === 'all' || config.target_monitor === m.id;

            return (
              <div
                key={m.id}
                className={`p-4 border transition-all relative ${
                  m.is_hdr_enabled
                    ? 'bg-[#180e10] border-[#f55a6b] neon-glow-coral'
                    : 'bg-[#130e0f] border-[#f55a6b]/30 hover:border-[#f55a6b]/70'
                }`}
              >
                <div className="absolute inset-0 scanlines-overlay opacity-20 pointer-events-none" />

                <div className="flex items-start justify-between relative z-10">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4 className="font-bold text-sm text-white truncate max-w-[220px]" title={m.name}>
                        {m.name}
                      </h4>
                      {m.is_primary && (
                        <span className="text-[10px] px-1.5 py-0.5 bg-[#5accf5]/15 text-[#5accf5] border border-[#5accf5]/40 font-bold">
                          {t.displaysPrimary}
                        </span>
                      )}
                      {isTarget && (
                        <span className="text-[10px] px-1.5 py-0.5 bg-[#f55a6b]/15 text-[#f55a6b] border border-[#f55a6b]/40 font-bold">
                          {t.displaysTargetHdr}
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs">
                      {m.is_hdr_supported ? (
                        <span className="text-emerald-400 flex items-center gap-1 font-semibold">
                          <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                          {t.displaysHdrSupported}
                        </span>
                      ) : (
                        <span className="text-[#8a7f81]">{t.displaysSdrOnly}</span>
                      )}
                      <span className="text-[#8a7f81]">•</span>
                      <span className="text-[#5accf5] text-xs font-mono">
                        {t.displaysTargetId}: {m.target_id}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {m.is_hdr_supported && (
                      <GlitchButton
                        label={m.is_hdr_enabled ? t.displaysHdrOn : t.displaysSdr}
                        variant={m.is_hdr_enabled ? 'primary' : 'outline'}
                        size="sm"
                        onClick={() => handleToggleMonitor(m)}
                      />
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* POSLEDNÍ HRY & HOOK TELEMETRIE */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <History className="w-4 h-4 text-[#f55a6b]" />
            <h3 className="font-bold text-xs uppercase tracking-wider text-[#f55a6b]">
              {t.recentTitle}
            </h3>
            <span className="text-xs text-[#8a7f81]">({recentGames.length})</span>
          </div>

          <button
            onClick={onNavigateToApps}
            className="text-xs text-[#5accf5] hover:text-[#70d6f7] flex items-center gap-1 font-bold cursor-pointer uppercase transition-colors"
          >
            <span>{t.recentAllLibrary(config.apps.length)}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3.5">
          {recentGames.slice(0, 6).map((game) => {
            const steamCover = game.steam_id
              ? `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${game.steam_id}/library_600x900.jpg`
              : null;

            const isHookActive = game.hook_status === 'active';

            // Support Tier label colors
            const getTierBadge = () => {
              if (game.hdr_type === 'autohdr') {
                return (
                  <span className="px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider bg-purple-950/80 text-purple-300 border border-purple-500/40">
                    {t.recentTierAutoHdr}
                  </span>
                );
              }
              if (game.hdr_type === 'mod' || game.hdr_type === 'custom') {
                return (
                  <span className="px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider bg-amber-950/80 text-amber-300 border border-amber-500/40">
                    {t.recentTierMod}
                  </span>
                );
              }
              return (
                <span className="px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider bg-cyan-950/80 text-[#5accf5] border border-[#5accf5]/50">
                  {t.recentTierNative}
                </span>
              );
            };

            return (
              <div
                key={game.exe}
                className={`relative group border overflow-hidden flex flex-col justify-between transition-all duration-200 ${
                  isHookActive
                    ? 'bg-[#1c0f12] border-[#f55a6b] neon-glow-coral'
                    : 'bg-[#120d0e] border-[#f55a6b]/30 hover:border-[#f55a6b] hover:shadow-[0_0_15px_rgba(245,90,107,0.3)]'
                }`}
                style={{ height: '230px' }}
              >
                {/* Poster Artwork with Scanlines */}
                <div className="absolute inset-0">
                  {steamCover ? (
                    <img
                      src={steamCover}
                      alt={game.name}
                      className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                      onError={(e) => {
                        (e.target as HTMLElement).style.display = 'none';
                      }}
                    />
                  ) : (
                    <div className="w-full h-full bg-gradient-to-b from-[#221314] to-[#0f0b0b]" />
                  )}
                  {/* CRT Scanline overlay on image */}
                  <div className="absolute inset-0 scanlines-overlay opacity-35 pointer-events-none" />
                  <div className="absolute inset-0 bg-gradient-to-t from-[#0f0b0b] via-[#0f0b0b]/60 to-transparent pointer-events-none" />
                </div>

                {/* Top Badge: HDR Support Type */}
                <div className="relative z-10 p-2 flex items-center justify-between">
                  {getTierBadge()}
                  {game.launcher && (
                    <span className="px-1 py-0.2 text-[8px] font-mono text-[#b5a9ac] bg-black/60 border border-white/10 uppercase">
                      {game.launcher}
                    </span>
                  )}
                </div>

                {/* Bottom Overlay: Title & Hook Telemetry Status */}
                <div className="relative z-10 p-2.5 space-y-1.5 bg-[#0f0b0b]/90 border-t border-[#f55a6b]/20">
                  <div className="font-bold text-xs truncate text-white">
                    <GlitchText text={game.name} scrambleOnHover={true} />
                  </div>

                  {/* Hook Verification Badge */}
                  <div className="space-y-0.5">
                    <div className="flex items-center gap-1.5 text-[10px]">
                      <span
                        className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                          isHookActive
                            ? 'bg-[#5accf5] animate-status-pulse'
                            : 'bg-emerald-400'
                        }`}
                      />
                      <span
                        className={`truncate font-semibold ${
                          isHookActive ? 'text-[#5accf5]' : 'text-emerald-300'
                        }`}
                      >
                        {isHookActive ? t.recentHookActive : t.recentHookTriggered}
                      </span>
                    </div>

                    <div className="flex items-center justify-between text-[9px] text-[#8a7f81]">
                      <span>{game.last_switched_at}</span>
                      <span className="text-[#5accf5]">{t.recentHdrOk}</span>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Activity Log (Real-time CRT System Event Feed) */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-[#5accf5]" />
          <h3 className="font-bold text-xs uppercase tracking-wider text-[#f55a6b]">
            {t.activityTitle}
          </h3>
        </div>

        <div className="p-3.5 border border-[#f55a6b]/30 bg-[#0f0b0b] relative space-y-2 max-h-[160px] overflow-y-auto">
          <div className="absolute inset-0 scanlines-overlay opacity-20 pointer-events-none" />

          {activityLogs.map((log) => (
            <div
              key={log.id}
              className="flex items-center gap-2 text-xs font-mono transition-colors hover:text-white relative z-10"
            >
              <span className="text-[#8a7f81] shrink-0">[{log.timestamp}]</span>
              <span
                className={`w-1.5 h-1.5 shrink-0 ${
                  log.type === 'hdr_on'
                    ? 'bg-[#f55a6b]'
                    : log.type === 'hdr_off'
                    ? 'bg-amber-400'
                    : log.type === 'game'
                    ? 'bg-[#5accf5]'
                    : 'bg-emerald-400'
                }`}
              />
              <span className="text-[#d8cfd1] truncate">&gt; {log.message}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
