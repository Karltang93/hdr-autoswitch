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
      setSaveMessage('Nastavení bylo uloženo');
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
    <div className="space-y-6 max-w-3xl animate-fadeIn">
      {saveMessage && (
        <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span>{saveMessage}</span>
        </div>
      )}

      {/* Target Monitor */}
      <div
        className={`p-5 rounded-2xl border glass-panel corner-brackets space-y-3 ${
          isDark ? 'bg-[#0b0f19]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="flex items-center gap-2">
          <Monitor className="w-5 h-5 text-cyan-500" />
          <div>
            <h3 className="text-sm font-bold">Cílový monitor pro HDR</h3>
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
          className={`w-full p-2.5 text-sm rounded-xl border ${
            isDark
              ? 'bg-slate-800 border-white/10 text-white'
              : 'bg-white border-slate-300 text-slate-900'
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
        className={`p-5 rounded-2xl border glass-panel corner-brackets space-y-3 ${
          isDark ? 'bg-[#0b0f19]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="flex items-center gap-2">
          <Clock className="w-5 h-5 text-amber-500" />
          <div>
            <h3 className="text-sm font-bold">Zpoždění při Alt+Tab (Debounce)</h3>
            <p className="text-xs text-slate-400">
              Doba čekání před vypnutím HDR, když přepnete do jiné aplikace (zabrání probliknutí obrazovky).
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 pt-1">
          {[
            { val: 0, label: '0s (Okamžitě)' },
            { val: 1, label: '1s' },
            { val: 2, label: '2s (Doporučeno)' },
            { val: 3, label: '3s' },
            { val: 5, label: '5s' },
          ].map((item) => (
            <button
              key={item.val}
              type="button"
              onClick={() =>
                handleSave({ ...config, alt_tab_delay_seconds: item.val })
              }
              className={`px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer border transition-all ${
                config.alt_tab_delay_seconds === item.val
                  ? 'bg-cyan-500/20 text-cyan-400 border-cyan-500/40 font-mono shadow-[0_0_10px_rgba(6,182,212,0.2)]'
                  : isDark
                  ? 'border-white/10 hover:bg-white/5 text-slate-400 font-mono'
                  : 'border-slate-200 hover:bg-slate-100 text-slate-700 font-mono'
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      {/* Toggles: Autostart & Notifications */}
      <div
        className={`p-5 rounded-2xl border glass-panel corner-brackets space-y-4 ${
          isDark ? 'bg-[#0b0f19]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
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
            <div className="w-11 h-6 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-500"></div>
          </label>
        </div>

        <div className="border-t border-white/5 pt-4 flex items-center justify-between">
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
                handleSave({ ...config, notifications_enabled: e.target.checked })
              }
              className="sr-only peer"
            />
            <div className="w-11 h-6 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-500"></div>
          </label>
        </div>
      </div>

      {/* Switch Method */}
      <div
        className={`p-5 rounded-2xl border glass-panel corner-brackets space-y-3 ${
          isDark ? 'bg-[#0b0f19]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="flex items-center gap-2">
          <Sliders className="w-5 h-5 text-cyan-500" />
          <div>
            <h3 className="text-sm font-bold">Metoda přepínání HDR</h3>
            <p className="text-xs text-slate-400">
              Vyberte mechanismus pro aktivaci HDR ve Windows.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
          <div
            onClick={() => handleSave({ ...config, switch_method: 'native' })}
            className={`p-3 rounded-xl border cursor-pointer transition-all ${
              config.switch_method === 'native'
                ? 'bg-cyan-500/10 border-cyan-500/40 text-cyan-400'
                : isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-400'
                : 'border-slate-200 hover:bg-slate-50 text-slate-700'
            }`}
          >
            <div className="font-semibold text-xs flex items-center justify-between">
              <span>Nativní Win32 API</span>
              <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-400">
                RECOMMENDED
              </span>
            </div>
            <p className="text-[11px] text-slate-400 mt-1">
              Přímo ovládá displej přes DisplayConfig. Bleskové a tiché.
            </p>
          </div>

          <div
            onClick={() => handleSave({ ...config, switch_method: 'shortcut' })}
            className={`p-3 rounded-xl border cursor-pointer transition-all ${
              config.switch_method === 'shortcut'
                ? 'bg-cyan-500/10 border-cyan-500/40 text-cyan-400'
                : isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-400'
                : 'border-slate-200 hover:bg-slate-50 text-slate-700'
            }`}
          >
            <div className="font-semibold text-xs">Simulace Win+Alt+B</div>
            <p className="text-[11px] text-slate-400 mt-1">
              Simuluje stisk standardní klávesové zkratky Windows Game Baru.
            </p>
          </div>
        </div>
      </div>

      {/* Blacklist / Exclusions */}
      <div
        className={`p-5 rounded-2xl border glass-panel corner-brackets space-y-3 ${
          isDark ? 'bg-[#0b0f19]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="flex items-center gap-2">
          <ShieldBan className="w-5 h-5 text-rose-500" />
          <div>
            <h3 className="text-sm font-bold">Vyloučené aplikace (Blacklist)</h3>
            <p className="text-xs text-slate-400">
              Tyto aplikace (např. webové prohlížeče jako Chrome či Discord) NIKDY nepřepnou systém do HDR.
            </p>
          </div>
        </div>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 pt-1">
          <input
            type="text"
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            placeholder="např. brave.exe"
            className={`flex-1 px-3 py-2 text-xs font-mono rounded-xl border ${
              isDark
                ? 'bg-slate-800 border-white/10 text-white'
                : 'bg-white border-slate-300 text-slate-900'
            }`}
          />
          <button
            type="submit"
            className="flex items-center gap-1 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-white text-xs font-semibold rounded-xl border border-white/10 cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" /> Přidat
          </button>
        </form>

        <div className="flex flex-wrap gap-1.5 max-h-36 overflow-y-auto pt-1">
          {config.blacklist.map((exe) => (
            <span
              key={exe}
              className={`inline-flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-lg border font-mono ${
                isDark
                  ? 'bg-slate-800/80 border-white/10 text-slate-300'
                  : 'bg-slate-100 border-slate-200 text-slate-800'
              }`}
            >
              <span>{exe}</span>
              <button
                type="button"
                onClick={() => handleRemoveBlacklist(exe)}
                className="text-slate-500 hover:text-rose-400 cursor-pointer"
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
