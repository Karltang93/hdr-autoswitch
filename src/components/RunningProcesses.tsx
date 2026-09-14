import React, { useState, useEffect } from 'react';
import { RunningProcessInfo, AppConfig, HdrApp } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, Search, Plus, Check, AppWindow, ShieldBan } from 'lucide-react';

interface RunningProcessesProps {
  config: AppConfig;
  onUpdateConfig: (newConfig: AppConfig) => void;
  isDark: boolean;
}

export const RunningProcesses: React.FC<RunningProcessesProps> = ({
  config,
  onUpdateConfig,
  isDark,
}) => {
  const [processes, setProcesses] = useState<RunningProcessInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [addingExe, setAddingExe] = useState<string | null>(null);

  const fetchProcesses = async () => {
    setLoading(true);
    try {
      const list: RunningProcessInfo[] = await invoke('get_running_processes');
      setProcesses(list);
    } catch (err) {
      console.error('Failed to get running processes:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchProcesses();
  }, []);

  const handleAddProcess = async (proc: RunningProcessInfo) => {
    setAddingExe(proc.exe_name);
    const newApp: HdrApp = {
      name: proc.name,
      exe_name: proc.exe_name.toLowerCase(),
      enabled: true,
      hdr_type: 'custom',
      path: proc.path,
    };

    try {
      await invoke('add_custom_app', { app: newApp });
      const refreshed: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshed);
    } catch (err) {
      console.error('Failed to add app:', err);
    } finally {
      setAddingExe(null);
    }
  };

  const filtered = processes.filter(
    (p) =>
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      p.exe_name.toLowerCase().includes(search.toLowerCase()) ||
      p.title.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="space-y-5">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-black tracking-tight text-white flex items-center gap-2">
            <span>Běžící okna a procesy</span>
            <span className="text-xs font-mono font-normal px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
              {processes.length} aktivních
            </span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Aktuálně spuštěná okna na ploše. Kliknutím na tlačítko zařadíte libovolnou hru ihned do sledování.
          </p>
        </div>

        <button
          onClick={fetchProcesses}
          disabled={loading}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl border text-xs font-bold cursor-pointer transition-all ${
            isDark
              ? 'border-white/10 hover:border-cyan-500/40 hover:bg-white/[0.04] text-slate-200 shadow-sm'
              : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-xs'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin text-cyan-400' : 'text-cyan-400'}`} />
          <span>{loading ? 'Skenuji...' : 'Obnovit okna'}</span>
        </button>
      </div>

      {/* Search Field */}
      <div className="relative">
        <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Hledat mezi běžícími aplikacemi v reálném čase..."
          className={`w-full pl-9 pr-4 py-2.5 text-xs md:text-sm rounded-xl border transition-all ${
            isDark
              ? 'bg-[#0e1322]/80 border-white/[0.08] focus:border-cyan-500/50 text-white placeholder-slate-500'
              : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400 shadow-xs'
          }`}
        />
      </div>

      {/* Running Processes List */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="divide-y divide-white/[0.05] max-h-[520px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-16 text-center text-slate-400 text-xs font-mono">
              {loading ? 'SKENUJI BĚŽÍCÍ PROCESY...' : 'NENALEZENY ŽÁDNÉ PROCESY.'}
            </div>
          ) : (
            filtered.map((proc) => {
              const exeLower = proc.exe_name.toLowerCase();
              const isAlreadyAdded = config.apps.some(
                (a) => a.exe_name.toLowerCase() === exeLower
              );
              const isBlacklisted = config.blacklist.some(
                (b) => b.toLowerCase() === exeLower
              );

              return (
                <div
                  key={`${proc.pid}-${proc.exe_name}`}
                  className={`p-3.5 flex items-center justify-between gap-4 transition-colors ${
                    isDark ? 'hover:bg-white/[0.03]' : 'hover:bg-slate-50/80'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`p-2.5 rounded-xl border shrink-0 ${
                        isDark ? 'bg-white/[0.04] border-white/10 text-cyan-400' : 'bg-slate-100 border-slate-200 text-cyan-600'
                      }`}
                    >
                      <AppWindow className="w-4 h-4" />
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h4 className="text-sm font-extrabold truncate text-slate-100">{proc.name}</h4>
                        <span className="text-xs text-slate-400 font-mono">
                          {proc.exe_name}
                        </span>
                        <span className="text-[10px] text-slate-500 font-mono">
                          PID: {proc.pid}
                        </span>
                      </div>
                      {proc.title && (
                        <p className="text-xs text-slate-400 truncate mt-0.5 max-w-[450px]">
                          {proc.title}
                        </p>
                      )}
                    </div>
                  </div>

                  <div className="shrink-0">
                    {isBlacklisted ? (
                      <span className="inline-flex items-center gap-1.5 text-xs font-semibold px-3 py-1.5 rounded-xl bg-slate-800 text-slate-400 border border-white/5">
                        <ShieldBan className="w-3.5 h-3.5 text-slate-500" />
                        Vyloučeno
                      </span>
                    ) : isAlreadyAdded ? (
                      <span className="inline-flex items-center gap-1.5 text-xs font-bold px-3 py-1.5 rounded-xl bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 neon-glow-emerald">
                        <Check className="w-4 h-4" /> SLEDOVÁNO
                      </span>
                    ) : (
                      <button
                        onClick={() => handleAddProcess(proc)}
                        disabled={addingExe === proc.exe_name}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white font-bold text-xs shadow-md neon-glow-cyan cursor-pointer transition-all duration-150 hover:scale-105 active:scale-95"
                      >
                        <Plus className="w-4 h-4 fill-current" />
                        <span>PŘIDAT DO HDR</span>
                      </button>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
