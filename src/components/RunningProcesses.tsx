import React, { useState, useEffect } from 'react';
import { RunningProcessInfo, AppConfig, HdrApp } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, Search, Plus, Check, ShieldAlert, AppWindow } from 'lucide-react';

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
    <div className="space-y-4 animate-fadeIn">
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat mezi běžícími aplikacemi..."
            className={`w-full pl-9 pr-4 py-2 text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-slate-900/60 border-white/10 focus:border-cyan-500 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400'
            }`}
          />
        </div>

        <button
          onClick={fetchProcesses}
          disabled={loading}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
            isDark
              ? 'border-white/10 hover:bg-white/5 text-slate-300'
              : 'border-slate-300 hover:bg-slate-100 text-slate-700'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          Obnovit seznam
        </button>
      </div>

      <p className="text-xs text-slate-400">
        Kliknutím na tlačítko <strong>+ Přidat do HDR</strong> zařadíte jakoukoli běžící hru či přehrávač okamžitě mezi sledované aplikace.
      </p>

      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-slate-900/60 border-white/10' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="divide-y divide-white/5 max-h-[480px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              {loading ? 'Hledám běžící okna...' : 'Nenalezeny žádné procesy.'}
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
                    isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/50'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`p-2 rounded-xl ${
                        isDark ? 'bg-slate-800 text-cyan-400' : 'bg-slate-100 text-cyan-600'
                      }`}
                    >
                      <AppWindow className="w-4 h-4" />
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <h4 className="text-sm font-semibold truncate">{proc.name}</h4>
                        <span className="text-xs text-slate-500 font-mono">
                          {proc.exe_name}
                        </span>
                      </div>
                      <p className="text-xs text-slate-400 truncate mt-0.5" title={proc.title}>
                        {proc.title}
                      </p>
                    </div>
                  </div>

                  <div className="shrink-0">
                    {isAlreadyAdded ? (
                      <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                        <Check className="w-3.5 h-3.5" /> V HDR seznamu
                      </span>
                    ) : isBlacklisted ? (
                      <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-slate-500/10 text-slate-400 border border-slate-500/20 font-medium">
                        <ShieldAlert className="w-3.5 h-3.5" /> Vyloučeno
                      </span>
                    ) : (
                      <button
                        onClick={() => handleAddProcess(proc)}
                        disabled={addingExe === proc.exe_name}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-700 text-white text-xs font-semibold shadow-sm cursor-pointer transition-all disabled:opacity-50"
                      >
                        <Plus className="w-3.5 h-3.5" />
                        {addingExe === proc.exe_name ? 'Přidávám...' : 'Přidat do HDR'}
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
