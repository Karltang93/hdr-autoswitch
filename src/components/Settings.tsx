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
  isDark,
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
        <p className={`text-xs mt-1 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
          {t.settingsSubtitle}
        </p>
      </div>

      {saveMessage && (
        <div className={`p-3 border text-xs flex items-center gap-2.5 ${
          isDark ? 'border-emerald-500/40 bg-[#120e10] text-emerald-400' : 'border-emerald-300 bg-emerald-50 text-emerald-800'
        }`}>
          <CheckCircle2 className={`w-4 h-4 shrink-0 ${isDark ? 'text-emerald-400' : 'text-emerald-600'}`} />
          <span>&gt; {saveMessage}</span>
        </div>
      )}

      {/* Language Selection Group */}
      <div className={`p-5 border relative space-y-4 ${
        isDark ? 'border-[#f55a6b]/30 bg-[#120d0e]' : 'border-slate-200 bg-white shadow-2xs'
      }`}>
        {isDark && <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />}

        <div className={`flex items-center gap-2 text-xs font-bold uppercase tracking-wider ${
          isDark ? 'text-[#f55a6b]' : 'text-[#e03e52]'
        }`}>
          <Globe className={`w-4 h-4 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'}`} />
          <span>{t.settingsLanguageTitle}</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 relative z-10 items-center">
          <div className={`text-xs ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
            {t.settingsLanguageDesc}
          </div>

          <div className="flex gap-2 justify-end">
            <button
              onClick={() => setLang('en')}
              className={`px-4 py-2 text-xs font-bold uppercase border cursor-pointer transition-all ${
                lang === 'en'
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                  : isDark
                    ? 'bg-[#0f0b0b] text-[#8a7f81] border-[#f55a6b]/30 hover:text-white'
                    : 'bg-slate-50 text-slate-700 border-slate-200 hover:border-slate-300 hover:text-slate-900 shadow-2xs'
              }`}
            >
              {t.settingsEnglish}
            </button>
            <button
              onClick={() => setLang('cs')}
              className={`px-4 py-2 text-xs font-bold uppercase border cursor-pointer transition-all ${
                lang === 'cs'
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                  : isDark
                    ? 'bg-[#0f0b0b] text-[#8a7f81] border-[#f55a6b]/30 hover:text-white'
                    : 'bg-slate-50 text-slate-700 border-slate-200 hover:border-slate-300 hover:text-slate-900 shadow-2xs'
              }`}
            >
              {t.settingsCzech}
            </button>
          </div>
        </div>
      </div>

      {/* Monitor & Switching Method Group */}
      <div className={`p-5 border relative space-y-4 ${
        isDark ? 'border-[#f55a6b]/30 bg-[#120d0e]' : 'border-slate-200 bg-white shadow-2xs'
      }`}>
        {isDark && <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />}

        <div className={`flex items-center gap-2 text-xs font-bold uppercase tracking-wider ${
          isDark ? 'text-[#f55a6b]' : 'text-[#e03e52]'
        }`}>
          <Monitor className={`w-4 h-4 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'}`} />
          <span>{t.settingsDisplayGroup}</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 relative z-10">
          <div className="space-y-1.5">
            <label className={`text-xs uppercase ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>{t.settingsTargetMonitor}</label>
            <select
              value={targetValue}
              onChange={(e) => handleTarget(e.target.value)}
              className={`w-full px-3 py-2 text-xs border focus:border-[#f55a6b] focus:outline-none ${
                isDark ? 'border-[#f55a6b]/30 bg-[#0f0b0b] text-white' : 'border-slate-300 bg-white text-slate-900 shadow-2xs'
              }`}
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
            <p className={`text-[11px] ${isDark ? 'text-[#8a7f81]' : 'text-slate-500'}`}>{t.configTargetHint}</p>
          </div>

          <div className="space-y-1.5">
            <label className={`text-xs uppercase ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>{t.settingsSwitchMethod}</label>
            {config.switch_method === 'native' ? (
              <p className={`text-xs py-2 ${isDark ? 'text-[#5accf5]' : 'text-sky-700 font-semibold'}`}>{t.settingsMethodNative}</p>
            ) : (
              <>
                <p className={`text-xs ${isDark ? 'text-amber-300' : 'text-amber-700 font-semibold'}`}>{t.configNativeConsent}</p>
                <button
                  className={`p-2 text-xs border ${
                    isDark ? 'border-[#5accf5]/50 text-[#5accf5]' : 'border-sky-400 text-sky-700 bg-sky-50'
                  }`}
                  onClick={() => handleSave({ switch_method: 'native' })}
                >
                  {t.configAcceptNative}
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Switching Policy Group (Alt+Tab vs Game Exit) */}
      <div className={`p-5 border relative space-y-4 ${
        isDark ? 'border-[#f55a6b]/30 bg-[#120d0e]' : 'border-slate-200 bg-white shadow-2xs'
      }`}>
        {isDark && <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />}

        <div className={`flex items-center gap-2 text-xs font-bold uppercase tracking-wider ${
          isDark ? 'text-[#f55a6b]' : 'text-[#e03e52]'
        }`}>
          <Clock className={`w-4 h-4 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'}`} />
          <span>{t.settingsSwitchingPolicyTitle}</span>
        </div>

        <p className={`text-xs relative z-10 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
          {t.settingsSwitchingPolicyDesc}
        </p>

        <div className="space-y-3 relative z-10">
          {/* Option 1: Exit Only */}
          <div
            onClick={() => handleSave({ exit_only_hdr: true })}
            className={`p-3.5 border cursor-pointer transition-all ${
              config.exit_only_hdr
                ? isDark
                  ? 'border-[#f55a6b] bg-[#1a0f12] neon-glow-coral'
                  : 'border-[#f55a6b] bg-rose-50/70 shadow-2xs'
                : isDark
                  ? 'border-white/10 bg-black/40 hover:border-white/20'
                  : 'border-slate-200 bg-slate-50/70 hover:border-slate-300 shadow-2xs'
            }`}
          >
            <div className="flex items-center justify-between">
              <div className={`font-bold text-xs flex items-center gap-2 ${isDark ? 'text-white' : 'text-slate-900'}`}>
                <span className={config.exit_only_hdr ? 'text-[#f55a6b]' : isDark ? 'text-slate-500' : 'text-slate-400'}>
                  {config.exit_only_hdr ? '●' : '○'}
                </span>
                <span>{t.settingsPolicyExitOnly}</span>
              </div>
              <span className={`text-[10px] px-2 py-0.5 border font-bold ${
                isDark
                  ? 'border-emerald-500/40 text-emerald-400 bg-emerald-500/10'
                  : 'border-emerald-300 text-emerald-800 bg-emerald-50'
              }`}>
                {t.settingsNoFlicker}
              </span>
            </div>
            <p className={`text-[11px] mt-1.5 pl-4 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
              {t.settingsPolicyExitOnlyDesc}
            </p>
          </div>

          {/* Option 2: Alt+Tab Debounce */}
          <div
            onClick={() => handleSave({ exit_only_hdr: false })}
            className={`p-3.5 border cursor-pointer transition-all ${
              !config.exit_only_hdr
                ? isDark
                  ? 'border-[#f55a6b] bg-[#1a0f12] neon-glow-coral'
                  : 'border-[#f55a6b] bg-rose-50/70 shadow-2xs'
                : isDark
                  ? 'border-white/10 bg-black/40 hover:border-white/20'
                  : 'border-slate-200 bg-slate-50/70 hover:border-slate-300 shadow-2xs'
            }`}
          >
            <div className="flex items-center justify-between">
              <div className={`font-bold text-xs flex items-center gap-2 ${isDark ? 'text-white' : 'text-slate-900'}`}>
                <span className={!config.exit_only_hdr ? 'text-[#f55a6b]' : isDark ? 'text-slate-500' : 'text-slate-400'}>
                  {!config.exit_only_hdr ? '●' : '○'}
                </span>
                <span>{t.settingsPolicyAltTab}</span>
              </div>
            </div>
            <p className={`text-[11px] mt-1.5 pl-4 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
              {t.settingsPolicyAltTabDesc}
            </p>
          </div>
        </div>

        {/* Debounce slider (only active if Alt+Tab mode is chosen) */}
        {!config.exit_only_hdr && (
          <div className={`space-y-3 pt-3 border-t relative z-10 ${isDark ? 'border-white/10' : 'border-slate-200'}`}>
            <div className="flex items-center justify-between text-xs">
              <span className={isDark ? 'text-[#8a7f81]' : 'text-slate-600'}>{t.settingsDebounceLabel}</span>
              <span className={`font-bold px-2 py-0.5 border ${
                isDark ? 'text-[#5accf5] border-[#5accf5]/40 bg-black' : 'text-sky-700 border-sky-300 bg-sky-50'
              }`}>
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

            <p className={`text-[11px] ${isDark ? 'text-[#8a7f81]' : 'text-slate-500'}`}>
              {t.settingsDebounceDesc}
            </p>
          </div>
        )}
      </div>

      {/* System Integration Group */}
      <div className={`p-5 border relative space-y-4 ${
        isDark ? 'border-[#f55a6b]/30 bg-[#120d0e]' : 'border-slate-200 bg-white shadow-2xs'
      }`}>
        {isDark && <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />}

        <div className={`flex items-center gap-2 text-xs font-bold uppercase tracking-wider ${
          isDark ? 'text-[#f55a6b]' : 'text-[#e03e52]'
        }`}>
          <Power className={`w-4 h-4 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'}`} />
          <span>{t.settingsSystemGroup}</span>
        </div>

        <div className="space-y-3 relative z-10">
          {[
            {
              title: t.settingsAutostartTitle,
              desc: t.settingsAutostartDesc,
              active: config.autostart,
              toggle: () => handleSave({ autostart: !config.autostart }),
            },
            {
              title: t.settingsStartMinimizedTitle,
              desc: t.settingsStartMinimizedDesc,
              active: config.start_minimized,
              toggle: () => handleSave({ start_minimized: !config.start_minimized }),
            },
            {
              title: t.settingsAutoDetectTitle,
              desc: t.settingsAutoDetectDesc,
              active: config.auto_detect_new_games,
              toggle: () => handleSave({ auto_detect_new_games: !config.auto_detect_new_games }),
            },
            {
              title: t.settingsAutoSyncTitle,
              desc: t.settingsAutoSyncDesc,
              active: config.auto_sync_database,
              toggle: () => handleSave({ auto_sync_database: !config.auto_sync_database }),
            },
            {
              title: t.settingsNotifTitle,
              desc: t.settingsNotifDesc,
              active: config.notifications_enabled,
              toggle: () => handleSave({ notifications_enabled: !config.notifications_enabled }),
            },
          ].map((item, idx) => (
            <div
              key={idx}
              className={`flex items-center justify-between p-3 border ${
                isDark ? 'border-white/10 bg-black/40' : 'border-slate-200 bg-slate-50/70 shadow-2xs'
              }`}
            >
              <div className="space-y-0.5">
                <div className={`font-bold text-xs ${isDark ? 'text-white' : 'text-slate-900'}`}>{item.title}</div>
                <div className={`text-[11px] ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
                  {item.desc}
                </div>
              </div>

              <button
                onClick={item.toggle}
                className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                  item.active
                    ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                    : isDark
                      ? 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
                      : 'bg-white text-slate-600 border-slate-300 hover:text-slate-900 shadow-2xs'
                }`}
              >
                {item.active ? t.settingsStateOn : t.settingsStateOff}
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Blacklist Group */}
      <div className={`p-5 border relative space-y-4 ${
        isDark ? 'border-[#f55a6b]/30 bg-[#120d0e]' : 'border-slate-200 bg-white shadow-2xs'
      }`}>
        {isDark && <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />}

        <div className={`flex items-center gap-2 text-xs font-bold uppercase tracking-wider ${
          isDark ? 'text-[#f55a6b]' : 'text-[#e03e52]'
        }`}>
          <ShieldBan className={`w-4 h-4 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'}`} />
          <span>{t.settingsBlacklistGroup}</span>
        </div>

        <p className={`text-xs relative z-10 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
          {t.settingsBlacklistDesc}
        </p>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 relative z-10">
          <input
            type="text"
            placeholder={t.settingsBlacklistPlaceholder}
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            className={`flex-1 px-3 py-2 text-xs border focus:border-[#f55a6b] focus:outline-none ${
              isDark
                ? 'border-[#f55a6b]/30 bg-[#0f0b0b] text-white'
                : 'border-slate-300 bg-white text-slate-900 placeholder-slate-400 shadow-2xs'
            }`}
          />
          <GlitchButton
            type="submit"
            label={t.settingsBlacklistAddBtn}
            variant="primary"
            size="sm"
            isDark={isDark}
            icon={<Plus className="w-3.5 h-3.5 fill-current" />}
          />
        </form>

        <div className="space-y-1.5 relative z-10">
          {config.blacklist.length === 0 ? (
            <div className={`text-xs py-2 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>{t.settingsBlacklistEmpty}</div>
          ) : (
            config.blacklist.map((exe) => (
              <div
                key={exe}
                className={`p-2 border flex items-center justify-between text-xs ${
                  isDark ? 'border-white/10 bg-black/40' : 'border-slate-200 bg-slate-50/70 shadow-2xs'
                }`}
              >
                <span className={`font-mono ${isDark ? 'text-[#5accf5]' : 'text-sky-700 font-semibold'}`}>[{exe}]</span>
                <button
                  onClick={() => handleRemoveBlacklist(exe)}
                  className={`cursor-pointer ${isDark ? 'text-[#8a7f81] hover:text-[#f55a6b]' : 'text-slate-400 hover:text-[#f55a6b]'}`}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>
      {snapshot && <p className={`text-[11px] break-all ${isDark ? 'text-[#8a7f81]' : 'text-slate-500'}`}>{t.configFile}: {snapshot.config_path}</p>}
    </fieldset>
  );
};
