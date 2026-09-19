import { useState } from 'react';
import { AppConfig, MonitorInfo, SettingsPatch } from '../types';
import { configClient, useConfig } from '../useConfig';
import { monitorReady } from '../displayState';
import {
  Monitor,
  Clock,
  Power,
  ShieldBan,
  Plus,
  Trash2,
  CheckCircle2,
  Globe,
} from 'lucide-react';
import { GlitchButton } from './GlitchButton';
import { useI18n } from '../i18n';

interface SettingsProps {
  config: AppConfig;
  monitors: MonitorInfo[];
  isDark: boolean;
}

export const Settings: React.FC<SettingsProps> = ({
  config,
  monitors,
}) => {
  const { t, lang, setLang } = useI18n();
  const { snapshot, pending } = useConfig();
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [newBlacklistExe, setNewBlacklistExe] = useState('');

  const selectedMonitor = monitors.find((monitor) =>
    monitor.is_selected && monitorReady(monitor));
  const targetValue = config.target_monitor.kind === 'all'
    ? 'all'
    : selectedMonitor?.device_path ?? 'unavailable';

  const handleSave = async (patch: SettingsPatch) => {
    setSaveMessage(null);
    try {
      await configClient.patch(patch);
      setSaveMessage(t.settingsSavedMsg);
      setTimeout(() => setSaveMessage(null), 2500);
    } catch (err) {
      configClient.reportError(err);
    }
  };

  const handleTarget = (value: string) => {
    if (value === 'all') {
      void handleSave({ target_monitor: { kind: 'all' } });
      return;
    }
    const monitor = monitors.find((item) => item.device_path === value && monitorReady(item));
    if (!monitor || !monitorReady(monitor)) {
      configClient.reportError(t.configMonitorIdentityError);
      return;
    }
    void handleSave({
      target_monitor: { kind: 'monitor', device_path: monitor.device_path, display_name: monitor.name },
    });
  };

  const handleAddBlacklist = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBlacklistExe.trim()) return;
    let clean = newBlacklistExe.trim().toLowerCase();
    if (!clean.endsWith('.exe')) clean += '.exe';

    if (!config.blacklist.includes(clean)) {
      const updated = { blacklist: [...config.blacklist, clean] };
      handleSave(updated);
    }
    setNewBlacklistExe('');
  };

  const handleRemoveBlacklist = (exe: string) => {
    const updated = {
      blacklist: config.blacklist.filter((b) => b !== exe),
    };
    handleSave(updated);
  };

  return (
    <fieldset disabled={pending} className={`space-y-6 max-w-4xl font-mono ${pending ? 'pointer-events-none opacity-70' : ''}`}>
      {/* Header */}
      <div>
        <div className="flex items-center gap-2.5">
          <h2 className="glitch-title-bar px-2.5 py-0.5 text-xs font-bold tracking-wider inline-block">
            {t.settingsTitle}
          </h2>
        </div>
        <p className="text-xs text-[#8a7f81] mt-1">
          {t.settingsSubtitle}
        </p>
      </div>

      {saveMessage && (
        <div className="p-3 border border-emerald-500/40 bg-[#120e10] text-emerald-400 text-xs flex items-center gap-2.5">
          <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400" />
          <span>&gt; {saveMessage}</span>
        </div>
      )}

      {/* Language Selection Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Globe className="w-4 h-4 text-[#5accf5]" />
          <span>{t.settingsLanguageTitle}</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 relative z-10 items-center">
          <div className="text-xs text-[#8a7f81]">
            {t.settingsLanguageDesc}
          </div>

          <div className="flex gap-2 justify-end">
            <button
              onClick={() => setLang('en')}
              className={`px-4 py-2 text-xs font-bold uppercase border cursor-pointer transition-all ${
                lang === 'en'
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                  : 'bg-[#0f0b0b] text-[#8a7f81] border-[#f55a6b]/30 hover:text-white'
              }`}
            >
              {t.settingsEnglish}
            </button>
            <button
              onClick={() => setLang('cs')}
              className={`px-4 py-2 text-xs font-bold uppercase border cursor-pointer transition-all ${
                lang === 'cs'
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                  : 'bg-[#0f0b0b] text-[#8a7f81] border-[#f55a6b]/30 hover:text-white'
              }`}
            >
              {t.settingsCzech}
            </button>
          </div>
        </div>
      </div>

      {/* Monitor & Switching Method Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Monitor className="w-4 h-4 text-[#5accf5]" />
          <span>{t.settingsDisplayGroup}</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 relative z-10">
          <div className="space-y-1.5">
            <label className="text-xs uppercase text-[#8a7f81]">{t.settingsTargetMonitor}</label>
            <select
              value={targetValue}
              onChange={(e) => handleTarget(e.target.value)}
              className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#0f0b0b] focus:border-[#f55a6b] text-white focus:outline-none"
            >
              <option value="all">{t.settingsAllMonitors}</option>
              {targetValue === 'unavailable' && <option value="unavailable" disabled>
                {config.target_monitor.kind === 'monitor' ? `${config.target_monitor.display_name}: ` : ''}
                {config.target_monitor.kind === 'needs_confirmation' ? t.configConfirmTarget : t.configMissingTarget}
              </option>}
              {monitors
                .filter(monitorReady)
                .map((m) => (
                  <option key={m.id} value={m.device_path ?? ''}>
                    {m.name} {m.is_primary ? `(${t.displaysPrimary})` : ''}
                  </option>
                ))}
            </select>
            <p className="text-[11px] text-[#8a7f81]">{t.configTargetHint}</p>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs uppercase text-[#8a7f81]">{t.settingsSwitchMethod}</label>
            {config.switch_method === 'native' ? <p className="text-xs text-[#5accf5] py-2">{t.settingsMethodNative}</p> : <>
              <p className="text-xs text-amber-300">{t.configNativeConsent}</p>
              <button className="p-2 text-xs border border-[#5accf5]/50 text-[#5accf5]" onClick={() => handleSave({ switch_method: 'native' })}>
                {t.configAcceptNative}
              </button>
            </>}
          </div>
        </div>
      </div>

      {/* Switching Policy Group (Alt+Tab vs Game Exit) */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Clock className="w-4 h-4 text-[#5accf5]" />
          <span>{t.settingsSwitchingPolicyTitle}</span>
        </div>

        <p className="text-xs text-[#8a7f81] relative z-10">
          {t.settingsSwitchingPolicyDesc}
        </p>

        <div className="space-y-3 relative z-10">
          {/* Option 1: Exit Only */}
          <div
            onClick={() => handleSave({ exit_only_hdr: true })}
            className={`p-3.5 border cursor-pointer transition-all ${
              config.exit_only_hdr
                ? 'border-[#f55a6b] bg-[#1a0f12] neon-glow-coral'
                : 'border-white/10 bg-black/40 hover:border-white/20'
            }`}
          >
            <div className="flex items-center justify-between">
              <div className="font-bold text-xs text-white flex items-center gap-2">
                <span className={config.exit_only_hdr ? 'text-[#f55a6b]' : 'text-slate-500'}>
                  {config.exit_only_hdr ? '●' : '○'}
                </span>
                <span>{t.settingsPolicyExitOnly}</span>
              </div>
              <span className="text-[10px] px-2 py-0.5 border border-emerald-500/40 text-emerald-400 bg-emerald-500/10 font-bold">
                {t.settingsNoFlicker}
              </span>
            </div>
            <p className="text-[11px] text-[#8a7f81] mt-1.5 pl-4">
              {t.settingsPolicyExitOnlyDesc}
            </p>
          </div>

          {/* Option 2: Alt+Tab Debounce */}
          <div
            onClick={() => handleSave({ exit_only_hdr: false })}
            className={`p-3.5 border cursor-pointer transition-all ${
              !config.exit_only_hdr
                ? 'border-[#f55a6b] bg-[#1a0f12] neon-glow-coral'
                : 'border-white/10 bg-black/40 hover:border-white/20'
            }`}
          >
            <div className="flex items-center justify-between">
              <div className="font-bold text-xs text-white flex items-center gap-2">
                <span className={!config.exit_only_hdr ? 'text-[#f55a6b]' : 'text-slate-500'}>
                  {!config.exit_only_hdr ? '●' : '○'}
                </span>
                <span>{t.settingsPolicyAltTab}</span>
              </div>
            </div>
            <p className="text-[11px] text-[#8a7f81] mt-1.5 pl-4">
              {t.settingsPolicyAltTabDesc}
            </p>
          </div>
        </div>

        {/* Debounce slider (only active if Alt+Tab mode is chosen) */}
        {!config.exit_only_hdr && (
          <div className="space-y-3 pt-3 border-t border-white/10 relative z-10">
            <div className="flex items-center justify-between text-xs">
              <span className="text-[#8a7f81]">{t.settingsDebounceLabel}</span>
              <span className="font-bold text-[#5accf5] px-2 py-0.5 border border-[#5accf5]/40 bg-black">
                {t.settingsDebounceSeconds(config.alt_tab_delay_seconds)}
              </span>
            </div>

            <input
              type="range"
              min="1"
              max="10"
              step="1"
              value={config.alt_tab_delay_seconds}
              onChange={(e) =>
                handleSave({ alt_tab_delay_seconds: parseInt(e.target.value, 10) })
              }
              className="w-full accent-[#f55a6b] cursor-pointer"
            />

            <p className="text-[11px] text-[#8a7f81]">
              {t.settingsDebounceDesc}
            </p>
          </div>
        )}
      </div>

      {/* System Integration Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Power className="w-4 h-4 text-[#5accf5]" />
          <span>{t.settingsSystemGroup}</span>
        </div>

        <div className="space-y-3 relative z-10">
          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">{t.settingsAutostartTitle}</div>
              <div className="text-[11px] text-[#8a7f81]">
                {t.settingsAutostartDesc}
              </div>
            </div>

            <button
              onClick={() => handleSave({ autostart: !config.autostart })}
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.autostart
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.autostart ? t.settingsStateOn : t.settingsStateOff}
            </button>
          </div>

          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">{t.settingsStartMinimizedTitle}</div>
              <div className="text-[11px] text-[#8a7f81]">
                {t.settingsStartMinimizedDesc}
              </div>
            </div>

            <button
              onClick={() =>
                handleSave({ start_minimized: !config.start_minimized })
              }
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.start_minimized
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.start_minimized ? t.settingsStateOn : t.settingsStateOff}
            </button>
          </div>

          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">{t.settingsAutoDetectTitle}</div>
              <div className="text-[11px] text-[#8a7f81]">
                {t.settingsAutoDetectDesc}
              </div>
            </div>

            <button
              onClick={() =>
                handleSave({ auto_detect_new_games: !config.auto_detect_new_games })
              }
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.auto_detect_new_games
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.auto_detect_new_games ? t.settingsStateOn : t.settingsStateOff}
            </button>
          </div>

          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">{t.settingsAutoSyncTitle}</div>
              <div className="text-[11px] text-[#8a7f81]">
                {t.settingsAutoSyncDesc}
              </div>
            </div>

            <button
              onClick={() =>
                handleSave({ auto_sync_database: !config.auto_sync_database })
              }
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.auto_sync_database
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.auto_sync_database ? t.settingsStateOn : t.settingsStateOff}
            </button>
          </div>

          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">{t.settingsNotifTitle}</div>
              <div className="text-[11px] text-[#8a7f81]">
                {t.settingsNotifDesc}
              </div>
            </div>

            <button
              onClick={() =>
                handleSave({ notifications_enabled: !config.notifications_enabled })
              }
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.notifications_enabled
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.notifications_enabled ? t.settingsStateOn : t.settingsStateOff}
            </button>
          </div>
        </div>
      </div>

      {/* Blacklist Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <ShieldBan className="w-4 h-4 text-[#5accf5]" />
          <span>{t.settingsBlacklistGroup}</span>
        </div>

        <p className="text-xs text-[#8a7f81] relative z-10">
          {t.settingsBlacklistDesc}
        </p>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 relative z-10">
          <input
            type="text"
            placeholder={t.settingsBlacklistPlaceholder}
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            className="flex-1 px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#0f0b0b] focus:border-[#f55a6b] text-white focus:outline-none"
          />
          <GlitchButton
            type="submit"
            label={t.settingsBlacklistAddBtn}
            variant="primary"
            size="sm"
            icon={<Plus className="w-3.5 h-3.5 fill-current" />}
          />
        </form>

        <div className="space-y-1.5 relative z-10">
          {config.blacklist.length === 0 ? (
            <div className="text-xs text-[#8a7f81] py-2">{t.settingsBlacklistEmpty}</div>
          ) : (
            config.blacklist.map((exe) => (
              <div
                key={exe}
                className="p-2 border border-white/10 bg-black/40 flex items-center justify-between text-xs"
              >
                <span className="font-mono text-[#5accf5]">[{exe}]</span>
                <button
                  onClick={() => handleRemoveBlacklist(exe)}
                  className="text-[#8a7f81] hover:text-[#f55a6b] cursor-pointer"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>
      {snapshot && <p className="text-[11px] text-[#8a7f81] break-all">{t.configFile}: {snapshot.config_path}</p>}
    </fieldset>
  );
};
