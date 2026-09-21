import React, { useState, useEffect } from 'react';
import { RunningProcessInfo, AppConfig, HdrApp } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { configClient } from '../useConfig';
import { RefreshCw, Search, Plus, Check, AppWindow } from 'lucide-react';
import { GlitchButton } from './GlitchButton';
import { GlitchText } from './GlitchText';
import { useI18n } from '../i18n';

interface RunningProcessesProps {
  config: AppConfig;
  isDark: boolean;
}

export const RunningProcesses: React.FC<RunningProcessesProps> = ({
  config,
  isDark,
}) => {
  const { t } = useI18n();
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
  }, [config.apps]);

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
      await configClient.mutate('add_custom_app', { app: newApp });
    } catch (err) {
      configClient.reportError(err);
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
    <div className="space-y-5 font-mono">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2.5">
            <h2 className="glitch-title-bar px-2.5 py-0.5 text-xs font-bold tracking-wider inline-block">
              {t.procTitle}
            </h2>
            <span className={`text-xs px-2 py-0.5 border ${
              isDark ? 'border-[#5accf5]/40 text-[#5accf5] bg-[#140e10]' : 'border-sky-300 text-sky-700 bg-sky-50 font-semibold'
            }`}>
              {t.procCountActive(processes.length)}
            </span>
          </div>
          <p className={`text-xs mt-1 ${isDark ? 'text-[#8a7f81]' : 'text-slate-600'}`}>
            {t.procSubtitle}
          </p>
        </div>

        <GlitchButton
          label={loading ? t.procRefreshingBtn : t.procRefreshBtn}
          variant="outline"
          size="sm"
          disabled={loading}
          isDark={isDark}
          icon={<RefreshCw className={`w-3.5 h-3.5 ${isDark ? 'text-[#5accf5]' : 'text-sky-600'} ${loading ? 'animate-spin' : ''}`} />}
          onClick={fetchProcesses}
        />
      </div>

      {/* Search Field */}
      <div className="relative">
        <Search className={`w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 ${isDark ? 'text-[#8a7f81]' : 'text-slate-400'}`} />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t.procSearchPlaceholder}
          className={`w-full pl-9 pr-4 py-2 text-xs border focus:border-[#f55a6b] focus:outline-none transition-all ${
            isDark
              ? 'border-[#f55a6b]/30 bg-[#120d0e] text-white placeholder-[#8a7f81]'
              : 'border-slate-300 bg-white text-slate-900 placeholder-slate-400 shadow-2xs'
          }`}
        />
      </div>

      {/* Process List */}
      {loading ? (
        <div className={`p-12 text-center border text-xs ${
          isDark ? 'border-[#f55a6b]/20 bg-[#120d0e] text-[#5accf5]' : 'border-slate-200 bg-white text-sky-700 shadow-2xs'
        }`}>
          {t.procLoading}
        </div>
      ) : filtered.length === 0 ? (
        <div className={`p-12 text-center border text-xs ${
          isDark ? 'border-[#f55a6b]/20 bg-[#120d0e] text-[#8a7f81]' : 'border-slate-200 bg-white text-slate-600 shadow-2xs'
        }`}>
          {t.procEmpty}
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((proc) => {
            const tracked = proc.tracked_primary != null;
            const isAdding = addingExe === proc.exe_name;

            return (
              <div
                key={`${proc.pid}-${proc.exe_name}`}
                className={`p-3 border transition-all flex items-center justify-between gap-4 relative ${
                  isDark
                    ? tracked
                      ? 'bg-[#180e10] border-[#f55a6b]/50'
                      : 'bg-[#120d0e] border-[#f55a6b]/20 hover:border-[#f55a6b]/60'
                    : tracked
                      ? 'bg-rose-50/50 border-[#f55a6b]/50 shadow-2xs'
                      : 'bg-white border-slate-200 hover:border-[#f55a6b] shadow-2xs'
                }`}
              >
                {isDark && <div className="absolute inset-0 scanlines-overlay opacity-10 pointer-events-none" />}

                <div className="flex items-center gap-3 min-w-0 relative z-10">
                  <div className={`p-2 border ${
                    isDark ? 'border-[#f55a6b]/30 bg-black text-[#5accf5]' : 'border-slate-200 bg-slate-100 text-sky-700'
                  }`}>
                    <AppWindow className="w-4 h-4" />
                  </div>

                  <div className="space-y-0.5 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className={`font-bold text-sm truncate max-w-[280px] ${isDark ? 'text-white' : 'text-slate-900'}`}>
                        <GlitchText text={proc.name} scrambleOnHover={true} />
                      </span>
                      <span className={`text-[10px] px-1.5 py-0.2 font-mono border ${
                        isDark ? 'bg-black border-white/10 text-[#8a7f81]' : 'bg-slate-100 border-slate-300 text-slate-600'
                      }`}>
                        PID: {proc.pid}
                      </span>
                    </div>

                    {proc.title && proc.title !== proc.name && (
                      <div className={`text-xs truncate max-w-[450px] ${isDark ? 'text-[#8a7f81]' : 'text-slate-500'}`}>
                        "{proc.title}"
                      </div>
                    )}

                    <div className={`text-xs font-mono truncate ${isDark ? 'text-[#5accf5]' : 'text-sky-700 font-semibold'}`}>
                      [{proc.exe_name}]
                    </div>
                  </div>
                </div>

                <div className="shrink-0 relative z-10">
                  {tracked ? (
                    <span className={`inline-flex items-center gap-1.5 px-3 py-1 text-xs font-bold uppercase border ${
                      isDark ? 'bg-emerald-950/80 text-emerald-300 border-emerald-500/40' : 'bg-emerald-50 text-emerald-800 border-emerald-300'
                    }`}>
                      <Check className={`w-3.5 h-3.5 ${isDark ? 'text-emerald-300' : 'text-emerald-600'}`} /> {t.procAlreadyTracked}
                    </span>
                  ) : (
                    <GlitchButton
                      label={isAdding ? t.procAdding : t.procAddToHdr}
                      variant="primary"
                      size="sm"
                      disabled={isAdding}
                      isDark={isDark}
                      icon={<Plus className="w-3.5 h-3.5 fill-current" />}
                      onClick={() => handleAddProcess(proc)}
                    />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
