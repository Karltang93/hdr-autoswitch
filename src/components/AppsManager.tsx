import React, { useState } from 'react';
import { HdrApp, HdrType, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Search,
  Plus,
  RefreshCw,
  Gamepad2,
  Zap,
  Film,
  FolderPlus,
  Trash2,
  CheckCircle2,
  XCircle,
} from 'lucide-react';

interface AppsManagerProps {
  config: AppConfig;
  onUpdateConfig: (newConfig: AppConfig) => void;
  isDark: boolean;
}

export const AppsManager: React.FC<AppsManagerProps> = ({
  config,
  onUpdateConfig,
  isDark,
}) => {
  const [search, setSearch] = useState('');
  const [selectedFilter, setSelectedFilter] = useState<'all' | HdrType>('all');
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<string | null>(null);

  // Modal for adding custom app
  const [showAddModal, setShowAddModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newExe, setNewExe] = useState('');
  const [newType, setNewType] = useState<HdrType>('custom');

  const handleToggleApp = async (exeName: string, enabled: boolean) => {
    try {
      await invoke('toggle_app', { exeName, enabled });
      const updatedApps = config.apps.map((a) =>
        a.exe_name.toLowerCase() === exeName.toLowerCase() ? { ...a, enabled } : a
      );
      onUpdateConfig({ ...config, apps: updatedApps });
    } catch (err) {
      console.error('Failed to toggle app:', err);
    }
  };

  const handleDeleteApp = async (exeName: string) => {
    try {
      await invoke('remove_app', { exeName });
      const updatedApps = config.apps.filter(
        (a) => a.exe_name.toLowerCase() !== exeName.toLowerCase()
      );
      onUpdateConfig({ ...config, apps: updatedApps });
    } catch (err) {
      console.error('Failed to remove app:', err);
    }
  };

  const handleSyncDatabase = async () => {
    setIsSyncing(true);
    setSyncResult(null);
    try {
      const added: number = await invoke('sync_database');
      const refreshedConfig: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshedConfig);
      setSyncResult(
        added > 0
          ? `Úspěšně přidáno ${added} nových HDR her z komunitní databáze!`
          : 'Databáze je již plně aktuální!'
      );
    } catch (err) {
      console.error('Failed to sync database:', err);
      setSyncResult('Nepodařilo se připojit k online databázi.');
    } finally {
      setIsSyncing(false);
      setTimeout(() => setSyncResult(null), 5000);
    }
  };

  const handleAddCustomApp = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName.trim() || !newExe.trim()) return;

    let cleanExe = newExe.trim().toLowerCase();
    if (!cleanExe.endsWith('.exe')) {
      cleanExe += '.exe';
    }

    const newApp: HdrApp = {
      name: newName.trim(),
      exe_name: cleanExe,
      enabled: true,
      hdr_type: newType,
    };

    try {
      await invoke('add_custom_app', { app: newApp });
      const refreshedConfig: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshedConfig);
      setShowAddModal(false);
      setNewName('');
      setNewExe('');
      setNewType('custom');
    } catch (err) {
      console.error('Failed to add app:', err);
    }
  };

  const filteredApps = config.apps.filter((app) => {
    const matchesSearch =
      app.name.toLowerCase().includes(search.toLowerCase()) ||
      app.exe_name.toLowerCase().includes(search.toLowerCase());

    const matchesFilter =
      selectedFilter === 'all' ? true : app.hdr_type === selectedFilter;

    return matchesSearch && matchesFilter;
  });

  const getTypeBadge = (type: HdrType) => {
    switch (type) {
      case 'native':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-rose-500/10 text-rose-400 border border-rose-500/20 font-medium">
            <Gamepad2 className="w-3 h-3" /> Nativní HDR
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-medium">
            <Zap className="w-3 h-3" /> Auto HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 font-medium">
            <Film className="w-3 h-3" /> Přehrávač
          </span>
        );
      case 'custom':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20 font-medium">
            <FolderPlus className="w-3 h-3" /> Vlastní
          </span>
        );
    }
  };

  return (
    <div className="space-y-5 animate-fadeIn">
      {/* Action and Search Header */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat hru nebo proces (např. cyberpunk2077.exe)..."
            className={`w-full pl-9 pr-4 py-2 text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-slate-900/60 border-white/10 focus:border-cyan-500 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400'
            }`}
          />
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowAddModal(true)}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-700 text-white text-xs font-semibold shadow-md shadow-cyan-600/20 cursor-pointer transition-all"
          >
            <Plus className="w-4 h-4" /> Přidat aplikaci
          </button>

          <button
            onClick={handleSyncDatabase}
            disabled={isSyncing}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
              isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-300'
                : 'border-slate-300 hover:bg-slate-100 text-slate-700'
            } disabled:opacity-50`}
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isSyncing ? 'animate-spin' : ''}`} />
            {isSyncing ? 'Synchronizuji...' : 'Aktualizovat z webu'}
          </button>
        </div>
      </div>

      {syncResult && (
        <div className="p-3 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 text-xs flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span>{syncResult}</span>
        </div>
      )}

      {/* Category Filter Pills */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
        {[
          { id: 'all', label: `Vše (${config.apps.length})` },
          {
            id: 'native',
            label: `Nativní HDR (${config.apps.filter((a) => a.hdr_type === 'native').length})`,
          },
          {
            id: 'autohdr',
            label: `Auto HDR (${config.apps.filter((a) => a.hdr_type === 'autohdr').length})`,
          },
          {
            id: 'media',
            label: `Média (${config.apps.filter((a) => a.hdr_type === 'media').length})`,
          },
          {
            id: 'custom',
            label: `Vlastní (${config.apps.filter((a) => a.hdr_type === 'custom').length})`,
          },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setSelectedFilter(tab.id as any)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-all whitespace-nowrap ${
              selectedFilter === tab.id
                ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 font-semibold'
                : isDark
                ? 'bg-slate-900/40 text-slate-400 hover:text-white border border-transparent'
                : 'bg-white/60 text-slate-600 hover:text-slate-900 border border-slate-200'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Apps List */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-slate-900/60 border-white/10' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="divide-y divide-white/5 max-h-[440px] overflow-y-auto">
          {filteredApps.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              Nebyly nalezeny žádné aplikace odpovídající filtru.
            </div>
          ) : (
            filteredApps.map((app) => (
              <div
                key={app.exe_name}
                className={`p-3.5 flex items-center justify-between gap-4 transition-colors ${
                  isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/50'
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <button
                    onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                    className="cursor-pointer"
                  >
                    {app.enabled ? (
                      <CheckCircle2 className="w-5 h-5 text-emerald-400 hover:opacity-80 transition-opacity" />
                    ) : (
                      <XCircle className="w-5 h-5 text-slate-500 hover:opacity-80 transition-opacity" />
                    )}
                  </button>

                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <h4
                        className={`text-sm font-semibold truncate ${
                          app.enabled ? '' : 'text-slate-500 line-through'
                        }`}
                      >
                        {app.name}
                      </h4>
                      {getTypeBadge(app.hdr_type)}
                    </div>
                    <p className="text-xs text-slate-400 font-mono mt-0.5 truncate">
                      {app.exe_name}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3 shrink-0">
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={app.enabled}
                      onChange={(e) => handleToggleApp(app.exe_name, e.target.checked)}
                      className="sr-only peer"
                    />
                    <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-500"></div>
                  </label>

                  {app.hdr_type === 'custom' && (
                    <button
                      onClick={() => handleDeleteApp(app.exe_name)}
                      className="p-1.5 rounded-lg text-rose-400 hover:bg-rose-500/10 cursor-pointer transition-colors"
                      title="Smazat aplikaci"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Add Custom App Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn">
          <div
            className={`w-full max-w-md rounded-2xl p-6 border glass-panel shadow-2xl ${
              isDark
                ? 'bg-slate-900 border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <h3 className="text-lg font-bold">Přidat vlastní HDR aplikaci</h3>
            <p className="text-xs text-slate-400 mt-1">
              Zadejte název hry a název spustitelného souboru (.exe).
            </p>

            <form onSubmit={handleAddCustomApp} className="mt-4 space-y-4">
              <div>
                <label className="text-xs font-semibold block mb-1">
                  Název hry / aplikace:
                </label>
                <input
                  type="text"
                  required
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="např. Cyberpunk 2077"
                  className={`w-full px-3 py-2 text-sm rounded-xl border ${
                    isDark
                      ? 'bg-slate-800 border-white/10 text-white'
                      : 'bg-slate-50 border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="text-xs font-semibold block mb-1">
                  Název .exe souboru:
                </label>
                <input
                  type="text"
                  required
                  value={newExe}
                  onChange={(e) => setNewExe(e.target.value)}
                  placeholder="např. Cyberpunk2077.exe"
                  className={`w-full px-3 py-2 text-sm rounded-xl font-mono border ${
                    isDark
                      ? 'bg-slate-800 border-white/10 text-white'
                      : 'bg-slate-50 border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="text-xs font-semibold block mb-1">Typ HDR:</label>
                <select
                  value={newType}
                  onChange={(e) => setNewType(e.target.value as HdrType)}
                  className={`w-full px-3 py-2 text-sm rounded-xl border ${
                    isDark
                      ? 'bg-slate-800 border-white/10 text-white'
                      : 'bg-slate-50 border-slate-300 text-slate-900'
                  }`}
                >
                  <option value="native">Nativní HDR</option>
                  <option value="autohdr">Windows Auto HDR</option>
                  <option value="media">Přehrávač videa</option>
                  <option value="custom">Vlastní pravidlo</option>
                </select>
              </div>

              <div className="flex items-center justify-end gap-3 pt-2">
                <button
                  type="button"
                  onClick={() => setShowAddModal(false)}
                  className={`px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer border ${
                    isDark
                      ? 'border-white/10 hover:bg-white/5 text-slate-300'
                      : 'border-slate-300 hover:bg-slate-100 text-slate-700'
                  }`}
                >
                  Zrušit
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-cyan-600 hover:bg-cyan-700 text-white shadow-md shadow-cyan-600/20 cursor-pointer"
                >
                  Uložit aplikaci
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
