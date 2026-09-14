import { useState, useEffect } from 'react';
import { AppConfig, MonitorInfo } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import {
  Monitor,
  Clock,
  Power,
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
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div>
        <h2 className="text-xl font-black tracking-tight text-white flex items-center gap-2">
          <span>Nastavení aplikace</span>
        </h2>
        <p className="text-xs text-slate-400 mt-0.5">
          Přizpůsobte si chování automatického přepínání, debounce prodlevu i spouštění se systémem.
        </p>
      </div>

      {saveMessage && (
        <div className="p-3.5 rounded-2xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs flex items-center gap-2.5 shadow-sm">
          <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400" />
          <span className="font-medium">{saveMessage}</span>
        </div>
      )}

      {/* Group 1: Display & HDR Switching Engine */}
      <div
        className={`p-6 rounded-3xl border glass-panel space-y-5 ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Monitor className="w-5 h-5 text-cyan-400" />
          <h3 className="font-extrabold text-sm tracking-wide uppercase text-slate-200">
            Displej a metoda přepínání
          </h3>
        </div>

        {/* Target Monitor Dropdown */}
        <div className="space-y-2">
          <label className="block text-xs font-semibold text-slate-300">
            Cílový monitor pro HDR
          </label>
          <select
            value={config.target_monitor}
            onChange={(e) =>
              handleSave({ ...config, target_monitor: e.target.value })
            }
            className={`w-full p-3 text-xs md:text-sm rounded-xl border transition-colors ${
              isDark
                ? 'bg-[#14192b] border-white/10 text-white focus:border-cyan-500'
                : 'bg-white border-slate-300 text-slate-900 shadow-xs'
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

        {/* Switching Method Cards */}
        <div className="space-y-2 pt-1">
          <label className="block text-xs font-semibold text-slate-300">
            Mechanismus pro aktivaci HDR ve Windows
          </label>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
            <div
              onClick={() => handleSave({ ...config, switch_method: 'native' })}
              className={`p-4 rounded-2xl border cursor-pointer transition-all duration-150 ${
                config.switch_method === 'native'
                  ? 'bg-cyan-500/15 border-cyan-500/50 shadow-md neon-glow-cyan'
                  : isDark
                  ? 'bg-white/[0.02] border-white/[0.06] hover:border-white/[0.15]'
                  : 'bg-slate-50 border-slate-200'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-extrabold text-xs text-slate-100">Nativní Win32 API</span>
                <span className="text-[10px] font-bold px-2 py-0.5 rounded-md bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
                  Doporučeno
                </span>
              </div>
              <p className="text-xs text-slate-400 mt-1.5 leading-relaxed">
                Přímo ovládá displej přes systémové DisplayConfig API. Bleskové, tiché přepnutí bez nutnosti simulace stisku klávesnice.
              </p>
            </div>

            <div
              onClick={() => handleSave({ ...config, switch_method: 'shortcut' })}
              className={`p-4 rounded-2xl border cursor-pointer transition-all duration-150 ${
                config.switch_method === 'shortcut'
                  ? 'bg-cyan-500/15 border-cyan-500/50 shadow-md neon-glow-cyan'
                  : isDark
                  ? 'bg-white/[0.02] border-white/[0.06] hover:border-white/[0.15]'
                  : 'bg-slate-50 border-slate-200'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-extrabold text-xs text-slate-100">Simulace Win+Alt+B</span>
              </div>
              <p className="text-xs text-slate-400 mt-1.5 leading-relaxed">
                Simuluje stisk klávesové zkratky Windows Game Baru. Alternativa pro starší sestavení Windows.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Group 2: Timing & Debounce Settings */}
      <div
        className={`p-6 rounded-3xl border glass-panel space-y-5 ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Clock className="w-5 h-5 text-amber-400" />
          <h3 className="font-extrabold text-sm tracking-wide uppercase text-slate-200">
            Zpoždění při Alt+Tab (Debounce)
          </h3>
        </div>

        <p className="text-xs text-slate-400">
          Doba čekání před vypnutím HDR při přepnutí do jiné aplikace. Zabraňuje probliknutí nebo zhasnutí monitoru při rychlém přepínání oken.
        </p>

        <div className="flex items-center gap-2 flex-wrap">
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
              className={`px-4 py-2 rounded-xl text-xs font-bold cursor-pointer transition-all ${
                config.alt_tab_delay_seconds === item.val
                  ? 'bg-cyan-500 text-slate-950 font-extrabold shadow-md neon-glow-cyan'
                  : isDark
                  ? 'bg-white/[0.03] text-slate-400 hover:text-white border border-white/[0.06] hover:border-cyan-500/30'
                  : 'bg-slate-100 text-slate-600 hover:text-slate-900 border border-slate-200'
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      {/* Group 3: System Preferences */}
      <div
        className={`p-6 rounded-3xl border glass-panel space-y-5 ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <Power className="w-5 h-5 text-emerald-400" />
          <h3 className="font-extrabold text-sm tracking-wide uppercase text-slate-200">
            Systémové chování
          </h3>
        </div>

        <div className="divide-y divide-white/[0.05]">
          <div className="pb-4 flex items-center justify-between gap-4">
            <div>
              <h4 className="text-sm font-bold text-slate-100">Spustit při startu Windows</h4>
              <p className="text-xs text-slate-400 mt-0.5">
                Aplikace se tiše spustí v oznamovací oblasti (system tray) při přihlášení uživatele.
              </p>
            </div>

            <button
              onClick={() => handleAutostartToggle(!autostartActive)}
              className={`px-4 py-1.5 rounded-full text-xs font-bold transition-all cursor-pointer ${
                autostartActive
                  ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                  : 'bg-slate-800 text-slate-400 border border-white/5'
              }`}
            >
              {autostartActive ? 'Zapnuto' : 'Vypnuto'}
            </button>
          </div>

          <div className="pt-4 flex items-center justify-between gap-4">
            <div>
              <h4 className="text-sm font-bold text-slate-100">Windows Toast Notifikace</h4>
              <p className="text-xs text-slate-400 mt-0.5">
                Zobrazit systémové oznámení při automatické aktivaci nebo deaktivaci HDR.
              </p>
            </div>

            <button
              onClick={() =>
                handleSave({
                  ...config,
                  notifications_enabled: !config.notifications_enabled,
                })
              }
              className={`px-4 py-1.5 rounded-full text-xs font-bold transition-all cursor-pointer ${
                config.notifications_enabled
                  ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40'
                  : 'bg-slate-800 text-slate-400 border border-white/5'
              }`}
            >
              {config.notifications_enabled ? 'Zapnuto' : 'Vypnuto'}
            </button>
          </div>
        </div>
      </div>

      {/* Group 4: Blacklist */}
      <div
        className={`p-6 rounded-3xl border glass-panel space-y-4 ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="flex items-center gap-2.5">
          <ShieldBan className="w-5 h-5 text-rose-400" />
          <h3 className="font-extrabold text-sm tracking-wide uppercase text-slate-200">
            Černá listina aplikací (Blacklist)
          </h3>
        </div>

        <p className="text-xs text-slate-400">
          Aplikace na tomto seznamu nikdy nezapnou HDR (vhodné pro webové prohlížeče jako Chrome nebo Discord).
        </p>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 pt-1">
          <input
            type="text"
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            placeholder="např. chrome.exe, discord.exe..."
            className={`flex-1 px-4 py-2 text-xs rounded-xl border font-mono ${
              isDark
                ? 'bg-white/[0.04] border-white/10 text-white placeholder-slate-500 focus:border-cyan-500'
                : 'bg-white border-slate-300 text-slate-900 placeholder-slate-400'
            }`}
          />
          <button
            type="submit"
            className="flex items-center gap-1.5 px-4 py-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs cursor-pointer shadow-sm neon-glow-cyan"
          >
            <Plus className="w-4 h-4 fill-current" /> Přidat
          </button>
        </form>

        <div className="flex flex-wrap gap-2 pt-1">
          {config.blacklist.map((exe) => (
            <span
              key={exe}
              className="inline-flex items-center gap-2 text-xs font-mono px-3 py-1.5 rounded-xl bg-white/[0.04] text-slate-200 border border-white/[0.08]"
            >
              <span>{exe}</span>
              <button
                onClick={() => handleRemoveBlacklist(exe)}
                className="text-slate-400 hover:text-rose-400 cursor-pointer"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
};
