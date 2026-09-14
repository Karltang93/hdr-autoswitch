import { useState, useEffect } from 'react';
import { AppConfig, MonitorInfo } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import {
  Monitor,
  Clock,
  Bell,
  Power,
  Sliders,
  ShieldBan,
  Plus,
  Trash2,
  CheckCircle2,
} from 'lucide-react';

interface SettingsProps {
  config: AppConfig;
  monitors: MonitorInfo[];
  onUpdateConfig: (newConfig: AppConfig) => void;
  isDark: boolean;
}

export const Settings: React.FC<SettingsProps> = ({
  config,
  monitors,
  onUpdateConfig,
  isDark,
}) => {
  const [autostartActive, setAutostartActive] = useState(false);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [newBlacklistExe, setNewBlacklistExe] = useState('');

  useEffect(() => {
    isEnabled().then(setAutostartActive).catch(console.error);
  }, []);

  const handleSave = async (updated: AppConfig) => {
    try {
      await invoke('save_config', { config: updated });
      onUpdateConfig(updated);
      setSaveMessage('Nastavení bylo úspěšně uloženo');
      setTimeout(() => setSaveMessage(null), 2500);
    } catch (err) {
      console.error('Failed to save config:', err);
    }
  };

  const handleAutostartToggle = async (active: boolean) => {
    try {
      if (active) {
        await enable();
      } else {
        await disable();
      }
      setAutostartActive(active);
      const updated = { ...config, autostart: active };
      await handleSave(updated);
    } catch (err) {
      console.error('Failed to toggle autostart:', err);
    }
  };

  const handleAddBlacklist = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newBlacklistExe.trim()) return;
    let clean = newBlacklistExe.trim().toLowerCase();
    if (!clean.endsWith('.exe')) clean += '.exe';

    if (!config.blacklist.includes(clean)) {
      const updated = { ...config, blacklist: [...config.blacklist, clean] };
      handleSave(updated);
    }
    setNewBlacklistExe('');
  };

  const handleRemoveBlacklist = (exe: string) => {
    const updated = {
      ...config,
      blacklist: config.blacklist.filter((b) => b !== exe),
    };
    handleSave(updated);
  };

  return (
    <div className="space-y-4 max-w-3xl">
      {saveMessage && (
        <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span>{saveMessage}</span>
        </div>
      )}

      {/* Target Monitor Setting */}
      <div
        className={`p-5 rounded-2xl border glass-panel space-y-3 ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Monitor className="w-5 h-5 text-sky-400" />
          <div>
            <h3 className="text-sm font-semibold">Cílový monitor pro HDR</h3>
            <p className="text-xs text-slate-400">
              Vyberte, které displeje se mají při spuštění HDR hry přepnout.
            </p>
          </div>
        </div>

        <select
          value={config.target_monitor}
          onChange={(e) =>
            handleSave({ ...config, target_monitor: e.target.value })
          }
          className={`w-full p-2.5 text-xs md:text-sm rounded-xl border ${
            isDark
              ? 'bg-[#121624] border-white/10 text-white'
              : 'bg-white border-slate-300 text-slate-900 shadow-sm'
          }`}
        >
          <option value="all">Všechny HDR monitory (Doporučeno)</option>
          {monitors.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name} {m.is_hdr_supported ? '(Podporuje HDR)' : '(Pouze SDR)'}
            </option>
          ))}
        </select>
      </div>

      {/* Alt+Tab Delay Debounce */}
      <div
        className={`p-5 rounded-2xl border glass-panel space-y-3 ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Clock className="w-5 h-5 text-amber-400" />
          <div>
            <h3 className="text-sm font-semibold">Zpoždění při Alt+Tab (Debounce)</h3>
            <p className="text-xs text-slate-400">
              Doba čekání před vypnutím HDR při přepnutí do jiné aplikace (zabrání probliknutí obrazovky při rychlém Alt+Tab).
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {[
            { val: 0, label: '0s (Okamžitě)' },
            { val: 1, label: '1s' },
            { val: 2, label: '2s (Doporučeno)' },
            { val: 3, label: '3s' },
            { val: 5, label: '5s' },
          ].map((item) => (
            <button
              key={item.val}
              onClick={() =>
                handleSave({ ...config, alt_tab_delay_seconds: item.val })
              }
              className={`px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-all ${
                config.alt_tab_delay_seconds === item.val
                  ? 'bg-sky-500/15 text-sky-300 border border-sky-500/30 font-semibold'
                  : isDark
                  ? 'bg-white/[0.03] text-slate-400 hover:text-white border border-white/[0.06]'
                  : 'bg-slate-100 text-slate-600 hover:text-slate-900 border border-slate-200'
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      {/* System Preferences: Autostart & Notifications */}
      <div
        className={`p-5 rounded-2xl border glass-panel space-y-4 ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Power className="w-5 h-5 text-emerald-400" />
            <div>
              <h4 className="text-sm font-semibold">Spustit při startu Windows</h4>
              <p className="text-xs text-slate-400">
                Aplikace se automaticky spustí v systémové liště při přihlášení.
              </p>
            </div>
          </div>

          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              checked={autostartActive}
              onChange={(e) => handleAutostartToggle(e.target.checked)}
              className="sr-only peer"
            />
            <div className="w-10 h-6 bg-slate-700/80 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-sky-500"></div>
          </label>
        </div>

        <div className="border-t border-white/[0.04] pt-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Bell className="w-5 h-5 text-purple-400" />
            <div>
              <h4 className="text-sm font-semibold">Windows Toast Notifikace</h4>
              <p className="text-xs text-slate-400">
                Zobrazit oznámení při automatickém zapnutí nebo vypnutí HDR.
              </p>
            </div>
          </div>

          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              checked={config.notifications_enabled}
              onChange={(e) =>
                handleSave({
                  ...config,
                  notifications_enabled: e.target.checked,
                })
              }
              className="sr-only peer"
            />
            <div className="w-10 h-6 bg-slate-700/80 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-sky-500"></div>
          </label>
        </div>
      </div>

      {/* HDR Switching Method */}
      <div
        className={`p-5 rounded-2xl border glass-panel space-y-3 ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Sliders className="w-5 h-5 text-sky-400" />
          <div>
            <h3 className="text-sm font-semibold">Metoda přepínání HDR</h3>
            <p className="text-xs text-slate-400">
              Vyberte mechanismus pro aktivaci HDR ve Windows.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
          <div
            onClick={() => handleSave({ ...config, switch_method: 'native' })}
            className={`p-3.5 rounded-xl border cursor-pointer transition-all ${
              config.switch_method === 'native'
                ? 'bg-sky-500/10 border-sky-500/40 shadow-sm'
                : isDark
                ? 'bg-white/[0.02] border-white/[0.06] hover:border-white/[0.12]'
                : 'bg-slate-50 border-slate-200 hover:border-slate-300'
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="font-semibold text-xs text-slate-100">Nativní Win32 API</span>
              <span className="text-[10px] font-medium px-2 py-0.2 rounded-full bg-emerald-500/15 text-emerald-400">
                Doporučeno
              </span>
            </div>
            <p className="text-xs text-slate-400 mt-1">
              Přímo ovládá displej přes DisplayConfig. Bleskové a tiché přepnutí bez nutnosti emulace stisku kláves.
            </p>
          </div>

          <div
            onClick={() => handleSave({ ...config, switch_method: 'shortcut' })}
            className={`p-3.5 rounded-xl border cursor-pointer transition-all ${
              config.switch_method === 'shortcut'
                ? 'bg-sky-500/10 border-sky-500/40 shadow-sm'
                : isDark
                ? 'bg-white/[0.02] border-white/[0.06] hover:border-white/[0.12]'
                : 'bg-slate-50 border-slate-200 hover:border-slate-300'
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="font-semibold text-xs text-slate-100">Simulace Win+Alt+B</span>
            </div>
            <p className="text-xs text-slate-400 mt-1">
              Simuluje stisk standardní klávesové zkratky Windows Game Baru. Alternativa pro starší buildy Windows 10.
            </p>
          </div>
        </div>
      </div>

      {/* Blacklist Section */}
      <div
        className={`p-5 rounded-2xl border glass-panel space-y-3 ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <ShieldBan className="w-5 h-5 text-rose-400" />
          <div>
            <h3 className="text-sm font-semibold">Černá listina aplikací (Blacklist)</h3>
            <p className="text-xs text-slate-400">
              Aplikace na tomto seznamu nikdy nezapnou HDR, i kdyby byly v popředí (např. webový prohlížeč nebo průzkumník).
            </p>
          </div>
        </div>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 pt-1">
          <input
            type="text"
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            placeholder="např. chrome.exe, discord.exe..."
            className={`flex-1 px-3 py-1.5 text-xs rounded-xl border font-mono ${
              isDark
                ? 'bg-white/[0.03] border-white/[0.08] text-white placeholder-slate-500'
                : 'bg-white border-slate-300 text-slate-900 placeholder-slate-400'
            }`}
          />
          <button
            type="submit"
            className="flex items-center gap-1 px-3 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 border border-white/[0.08] text-xs font-medium cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" /> Přidat
          </button>
        </form>

        <div className="flex flex-wrap gap-2 pt-1">
          {config.blacklist.map((exe) => (
            <span
              key={exe}
              className="inline-flex items-center gap-1.5 text-xs font-mono px-2.5 py-1 rounded-lg bg-white/[0.03] text-slate-300 border border-white/[0.06]"
            >
              <span>{exe}</span>
              <button
                onClick={() => handleRemoveBlacklist(exe)}
                className="text-slate-400 hover:text-rose-400 cursor-pointer"
              >
                <Trash2 className="w-3 h-3" />
              </button>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
};
