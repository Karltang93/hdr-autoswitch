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
    <div className="space-y-4">
      {/* Top Search & Refresh Bar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat mezi běžícími okny a aplikacemi..."
            className={`w-full pl-9 pr-4 py-2 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0c0f18]/80 border-white/[0.08] focus:border-sky-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-sky-500 text-slate-900 placeholder-slate-400 shadow-sm'
            }`}
          />
        </div>

        <button
          onClick={fetchProcesses}
          disabled={loading}
          className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-medium cursor-pointer transition-all ${
            isDark
              ? 'border-white/[0.08] hover:bg-white/[0.04] text-slate-300'
              : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-sm'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin text-sky-400' : 'text-slate-400'}`} />
          <span>Obnovit okna</span>
        </button>
      </div>

      <p className="text-xs text-slate-400">
        Kliknutím na tlačítko <strong>Přidat do HDR</strong> začne aplikace toto okno automaticky sledovat.
      </p>

      {/* Running Processes List */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="divide-y divide-white/[0.04] max-h-[500px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-10 text-center text-slate-400 text-xs">
              {loading ? 'Skenuji běžící okna...' : 'Nenalezeny žádné procesy odpovídající hledání.'}
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
                    isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/70'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`p-2 rounded-xl border shrink-0 ${
                        isDark ? 'bg-white/[0.03] border-white/[0.06] text-sky-400' : 'bg-slate-100 border-slate-200 text-sky-600'
                      }`}
                    >
                      <AppWindow className="w-4 h-4" />
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h4 className="text-sm font-semibold truncate text-slate-100">{proc.name}</h4>
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
                      <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-slate-800 text-slate-400 border border-white/[0.05]">
                        <ShieldBan className="w-3 h-3 text-slate-500" />
                        Vyloučeno
                      </span>
                    ) : isAlreadyAdded ? (
                      <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/25 font-medium">
                        <Check className="w-3.5 h-3.5" /> Sledováno
                      </span>
                    ) : (
                      <button
                        onClick={() => handleAddProcess(proc)}
                        disabled={addingExe === proc.exe_name}
                        className="flex items-center gap-1 px-3 py-1 rounded-lg border border-white/[0.08] hover:border-sky-400/40 hover:bg-sky-500/10 text-slate-300 hover:text-sky-200 text-xs font-medium cursor-pointer transition-all duration-150"
                      >
                        <Plus className="w-3.5 h-3.5" />
                        <span>Přidat do HDR</span>
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
