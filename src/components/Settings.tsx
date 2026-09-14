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
import { GlitchButton } from './GlitchButton';

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
    <div className="space-y-6 max-w-4xl font-mono">
      {/* Header */}
      <div>
        <div className="flex items-center gap-2.5">
          <h2 className="glitch-title-bar px-2.5 py-0.5 text-xs font-bold tracking-wider inline-block">
            NASTAVENÍ APLIKACE
          </h2>
        </div>
        <p className="text-xs text-[#8a7f81] mt-1">
          Přizpůsobte si chování automatického přepínání, debounce prodlevu i spouštění se systémem.
        </p>
      </div>

      {saveMessage && (
        <div className="p-3 border border-emerald-500/40 bg-[#120e10] text-emerald-400 text-xs flex items-center gap-2.5">
          <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400" />
          <span>&gt; {saveMessage}</span>
        </div>
      )}

      {/* Monitor & Switching Method Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Monitor className="w-4 h-4 text-[#5accf5]" />
          <span>CÍLOVÝ DISPLEJ A METODA PŘEPÍNÁNÍ</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 relative z-10">
          <div className="space-y-1.5">
            <label className="text-xs uppercase text-[#8a7f81]">CÍLOVÝ MONITOR</label>
            <select
              value={config.target_monitor}
              onChange={(e) => handleSave({ ...config, target_monitor: e.target.value })}
              className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#0f0b0b] focus:border-[#f55a6b] text-white focus:outline-none"
            >
              <option value="all">Všechny HDR monitory současně</option>
              {monitors
                .filter((m) => m.is_hdr_supported)
                .map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} {m.is_primary ? '(Primární)' : ''}
                  </option>
                ))}
            </select>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs uppercase text-[#8a7f81]">METODA PŘEPÍNÁNÍ HDR</label>
            <select
              value={config.switch_method}
              onChange={(e) =>
                handleSave({ ...config, switch_method: e.target.value as 'native' | 'shortcut' })
              }
              className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#0f0b0b] focus:border-[#f55a6b] text-white focus:outline-none"
            >
              <option value="native">Nativní Windows DisplayConfig API (Doporučeno)</option>
              <option value="shortcut">Virtuální Win + Alt + B zkratka</option>
            </select>
          </div>
        </div>
      </div>

      {/* Debounce Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Clock className="w-4 h-4 text-[#5accf5]" />
          <span>PRODLEVA PŘI ALT+TAB (DEBOUNCE)</span>
        </div>

        <div className="space-y-3 relative z-10">
          <div className="flex items-center justify-between text-xs">
            <span className="text-[#8a7f81]">Čas před návratem do SDR po opuštění hry:</span>
            <span className="font-bold text-[#5accf5] px-2 py-0.5 border border-[#5accf5]/40 bg-black">
              {config.alt_tab_delay_seconds} SEKUND
            </span>
          </div>

          <input
            type="range"
            min="0"
            max="10"
            step="1"
            value={config.alt_tab_delay_seconds}
            onChange={(e) =>
              handleSave({ ...config, alt_tab_delay_seconds: parseInt(e.target.value) })
            }
            className="w-full accent-[#f55a6b] cursor-pointer"
          />

          <p className="text-[11px] text-[#8a7f81]">
            Zabraňuje nepříjemnému problikávání monitoru při rychlém přepínání oken (např. kontrola Discordu či prohlížeče).
          </p>
        </div>
      </div>

      {/* System Integration Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <Power className="w-4 h-4 text-[#5accf5]" />
          <span>SYSTÉMOVÁ INTEGRACE</span>
        </div>

        <div className="space-y-3 relative z-10">
          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">SPOUŠTĚT AUTOMATICKY SE SYSTÉMEM</div>
              <div className="text-[11px] text-[#8a7f81]">
                Aplikace se tiše spustí na pozadí do systémové lišty (tray) po startu Windows.
              </div>
            </div>

            <button
              onClick={() => handleAutostartToggle(!autostartActive)}
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                autostartActive
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {autostartActive ? 'ZAPNUTO' : 'VYPNUTO'}
            </button>
          </div>

          <div className="flex items-center justify-between p-3 border border-white/10 bg-black/40">
            <div className="space-y-0.5">
              <div className="font-bold text-xs text-white">WINDOWS NOTIFIKACE</div>
              <div className="text-[11px] text-[#8a7f81]">
                Zobrazovat decentní systémové oznámení při každém přepnutí HDR režimu.
              </div>
            </div>

            <button
              onClick={() =>
                handleSave({ ...config, notifications_enabled: !config.notifications_enabled })
              }
              className={`px-3 py-1 text-xs font-bold uppercase tracking-wider border cursor-pointer transition-all ${
                config.notifications_enabled
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
              }`}
            >
              {config.notifications_enabled ? 'ZAPNUTO' : 'VYPNUTO'}
            </button>
          </div>
        </div>
      </div>

      {/* Blacklist Group */}
      <div className="p-5 border border-[#f55a6b]/30 bg-[#120d0e] relative space-y-4">
        <div className="absolute inset-0 scanlines-overlay opacity-15 pointer-events-none" />

        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-[#f55a6b]">
          <ShieldBan className="w-4 h-4 text-[#5accf5]" />
          <span>BLOKOVANÉ APLIKACE (BLACKLIST)</span>
        </div>

        <p className="text-xs text-[#8a7f81] relative z-10">
          Procesy, které nikdy nesmí aktivovat HDR (např. prohlížeče, editory apod.):
        </p>

        <form onSubmit={handleAddBlacklist} className="flex gap-2 relative z-10">
          <input
            type="text"
            placeholder="např. chrome.exe nebo obs64.exe"
            value={newBlacklistExe}
            onChange={(e) => setNewBlacklistExe(e.target.value)}
            className="flex-1 px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#0f0b0b] focus:border-[#f55a6b] text-white focus:outline-none"
          />
          <GlitchButton
            type="submit"
            label="PŘIDAT"
            variant="primary"
            size="sm"
            icon={<Plus className="w-3.5 h-3.5 fill-current" />}
          />
        </form>

        <div className="space-y-1.5 relative z-10">
          {config.blacklist.length === 0 ? (
            <div className="text-xs text-[#8a7f81] py-2">&gt; Žádné blokované procesy.</div>
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
    </div>
  );
};
