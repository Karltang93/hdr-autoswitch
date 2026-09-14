import React, { useState } from 'react';
import { HdrApp, HdrType, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Search,
  Plus,
  Compass,
  Trash2,
  CheckCircle2,
  XCircle,
  ScanSearch,
  Sparkles,
  Gamepad2,
} from 'lucide-react';

interface AppsManagerProps {
  config: AppConfig;
  onUpdateConfig: (newConfig: AppConfig) => void;
  onNavigateToCatalog: () => void;
  isDark: boolean;
}

export const AppsManager: React.FC<AppsManagerProps> = ({
  config,
  onUpdateConfig,
  onNavigateToCatalog,
  isDark,
}) => {
  const [search, setSearch] = useState('');
  const [isScanning, setIsScanning] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  // Scan results review modal
  const [showScanModal, setShowScanModal] = useState(false);
  const [scannedGames, setScannedGames] = useState<HdrApp[]>([]);
  const [selectedToImport, setSelectedToImport] = useState<Record<string, boolean>>({});

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

  const handleStartScan = async () => {
    setIsScanning(true);
    setScanMessage(null);
    try {
      const detected: HdrApp[] = await invoke('scan_installed_games');
      setScannedGames(detected);

      // Pre-select items that are not yet in config.apps
      const initialSelected: Record<string, boolean> = {};
      for (const item of detected) {
        const isAlreadyAdded = config.apps.some(
          (a) =>
            a.name.toLowerCase() === item.name.toLowerCase() ||
            a.exe_name.toLowerCase() === item.exe_name.toLowerCase()
        );
        initialSelected[item.exe_name] = !isAlreadyAdded;
      }
      setSelectedToImport(initialSelected);
      setShowScanModal(true);
    } catch (err) {
      console.error('Failed to scan installed games:', err);
      setScanMessage('Chyba při prohledávání disků.');
    } finally {
      setIsScanning(false);
    }
  };

  const handleConfirmImport = async () => {
    const toImport = scannedGames.filter((g) => selectedToImport[g.exe_name]);
    if (toImport.length === 0) {
      setShowScanModal(false);
      return;
    }

    try {
      const addedCount: number = await invoke('import_detected_games', {
        detected: toImport,
      });
      const refreshed: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshed);
      setShowScanModal(false);
      setScanMessage(`Úspěšně přidáno ${addedCount} her do sledování.`);
      setTimeout(() => setScanMessage(null), 4000);
    } catch (err) {
      console.error('Failed to import games:', err);
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
      console.error('Failed to add custom app:', err);
    }
  };

  const filteredApps = config.apps.filter(
    (app) =>
      app.name.toLowerCase().includes(search.toLowerCase()) ||
      app.exe_name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="space-y-4">
      {/* Top Action Bar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        {/* Search Field */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat v mých sledovaných hrách..."
            className={`w-full pl-9 pr-4 py-2 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0c0f18]/80 border-white/[0.08] focus:border-sky-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-sky-500 text-slate-900 placeholder-slate-400 shadow-sm'
            }`}
          />
        </div>

        {/* Buttons Group */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleStartScan}
            disabled={isScanning}
            className="flex items-center gap-2 px-3.5 py-2 rounded-xl bg-sky-500 hover:bg-sky-400 text-slate-950 font-semibold text-xs transition-all shadow-sm cursor-pointer disabled:opacity-50"
          >
            <ScanSearch className={`w-3.5 h-3.5 ${isScanning ? 'animate-spin' : ''}`} />
            <span>{isScanning ? 'Prohledávám disky...' : 'Skenovat hry v PC'}</span>
          </button>

          <button
            onClick={() => setShowAddModal(true)}
            className={`flex items-center gap-1.5 px-3 py-2 rounded-xl border text-xs font-medium cursor-pointer transition-colors ${
              isDark
                ? 'border-white/[0.08] hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-sm'
            }`}
          >
            <Plus className="w-3.5 h-3.5 text-sky-400" />
            <span>Přidat ručně</span>
          </button>

          <button
            onClick={onNavigateToCatalog}
            className={`flex items-center gap-1.5 px-3 py-2 rounded-xl border text-xs font-medium cursor-pointer transition-colors ${
              isDark
                ? 'border-white/[0.08] hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-sm'
            }`}
          >
            <Compass className="w-3.5 h-3.5 text-purple-400" />
            <span>Katalog</span>
          </button>
        </div>
      </div>

      {scanMessage && (
        <div className="p-3 rounded-xl bg-sky-500/10 border border-sky-500/20 text-sky-300 text-xs flex items-center gap-2">
          <Sparkles className="w-4 h-4 shrink-0 text-sky-400" />
          <span>{scanMessage}</span>
        </div>
      )}

      {/* Installed Games List */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="divide-y divide-white/[0.04] max-h-[500px] overflow-y-auto">
          {filteredApps.length === 0 ? (
            <div className="p-10 text-center space-y-3">
              <div className="text-slate-400 text-sm">
                {search
                  ? 'Nenalezena žádná hra odpovídající hledání.'
                  : 'Zatím zde nemáte žádné přidané hry.'}
              </div>
              {!search && (
                <div className="flex items-center justify-center gap-3">
                  <button
                    onClick={handleStartScan}
                    className="px-4 py-2 rounded-xl text-xs font-semibold bg-sky-500 hover:bg-sky-400 text-slate-950 cursor-pointer shadow-sm"
                  >
                    🔍 Skenovat hry na discích
                  </button>
                  <button
                    onClick={onNavigateToCatalog}
                    className="px-4 py-2 rounded-xl text-xs font-medium border border-white/[0.08] hover:bg-white/[0.04] text-slate-300 cursor-pointer"
                  >
                    Procházet databázi her
                  </button>
                </div>
              )}
            </div>
          ) : (
            filteredApps.map((app) => (
              <div
                key={app.exe_name}
                className={`p-3.5 flex items-center justify-between gap-4 transition-colors ${
                  isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/70'
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <button
                    onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                    className="cursor-pointer shrink-0"
                    title={app.enabled ? 'Sledování aktivní' : 'Sledování pozastaveno'}
                  >
                    {app.enabled ? (
                      <CheckCircle2 className="w-5 h-5 text-emerald-400 hover:opacity-80 transition-opacity" />
                    ) : (
                      <XCircle className="w-5 h-5 text-slate-500 hover:opacity-80 transition-opacity" />
                    )}
                  </button>

                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4
                        className={`text-sm font-semibold truncate ${
                          app.enabled ? 'text-slate-100' : 'text-slate-500 line-through'
                        }`}
                      >
                        {app.name}
                      </h4>

                      {app.path && (
                        <span
                          className="text-[10px] px-2 py-0.2 rounded-md bg-sky-500/10 text-sky-300 border border-sky-500/20 font-medium truncate max-w-[250px]"
                          title={app.path}
                        >
                          Nainstalováno
                        </span>
                      )}

                      {/* Clean compact badge with tooltip instead of multi-line text dump */}
                      {app.alternate_exes && app.alternate_exes.length > 0 && (
                        <span
                          className="text-[10px] px-2 py-0.2 rounded-md bg-purple-500/10 text-purple-300 border border-purple-500/20 font-mono cursor-help"
                          title={`Alternativní spustitelné soubory:\n${app.alternate_exes.join('\n')}`}
                        >
                          +{app.alternate_exes.length} procesů
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2 text-xs text-slate-400 font-mono mt-0.5">
                      <span>{app.exe_name}</span>
                    </div>
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
                    <div className="w-9 h-5 bg-slate-700/80 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-sky-500"></div>
                  </label>

                  <button
                    onClick={() => handleDeleteApp(app.exe_name)}
                    className="p-1.5 rounded-lg text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 cursor-pointer transition-colors"
                    title="Odebrat z mých her"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Interactive Scan Review Modal */}
      {showScanModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-fadeIn">
          <div
            className={`w-full max-w-lg rounded-2xl p-6 border glass-panel shadow-2xl flex flex-col max-h-[82vh] ${
              isDark
                ? 'bg-[#0c0f18] border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <div className="flex items-center gap-2.5 mb-1">
              <ScanSearch className="w-5 h-5 text-sky-400" />
              <h3 className="text-base font-bold">Nalezené HDR hry v počítači</h3>
            </div>
            <p className="text-xs text-slate-400 mb-3">
              Nalezli jsme následující nainstalované hry s podporou HDR. Vyberte, které chcete automaticky sledovat.
            </p>

            <div className="my-2 divide-y divide-white/[0.04] overflow-y-auto flex-1 pr-1 border rounded-xl border-white/[0.06] p-1">
              {scannedGames.length === 0 ? (
                <div className="py-8 text-center text-slate-500 text-sm">
                  Nebyly nalezeny žádné nové podporované hry na discích.
                </div>
              ) : (
                scannedGames.map((game) => {
                  const isChecked = !!selectedToImport[game.exe_name];
                  const alreadyInApps = config.apps.some(
                    (a) =>
                      a.exe_name.toLowerCase() === game.exe_name.toLowerCase() ||
                      a.name.toLowerCase() === game.name.toLowerCase()
                  );

                  return (
                    <label
                      key={game.exe_name}
                      className={`flex items-center justify-between p-3 rounded-lg cursor-pointer transition-colors ${
                        alreadyInApps
                          ? 'opacity-60 bg-white/[0.01]'
                          : isDark
                          ? 'hover:bg-white/[0.03]'
                          : 'hover:bg-slate-50'
                      }`}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <input
                          type="checkbox"
                          checked={isChecked}
                          disabled={alreadyInApps}
                          onChange={(e) =>
                            setSelectedToImport({
                              ...selectedToImport,
                              [game.exe_name]: e.target.checked,
                            })
                          }
                          className="w-4 h-4 rounded text-sky-500 border-slate-600 focus:ring-sky-500 focus:ring-offset-0 cursor-pointer"
                        />
                        <div className="min-w-0">
                          <div className="font-semibold text-xs truncate flex items-center gap-2">
                            <span>{game.name}</span>
                            {alreadyInApps && (
                              <span className="text-[10px] text-emerald-400">
                                (již přidáno)
                              </span>
                            )}
                          </div>
                          <div className="text-[11px] text-slate-400 font-mono">
                            {game.exe_name}
                          </div>
                        </div>
                      </div>
                    </label>
                  );
                })
              )}
            </div>

            <div className="flex items-center justify-between mt-4 pt-3 border-t border-white/[0.06]">
              <span className="text-xs text-slate-400">
                Vybráno:{' '}
                <strong className="text-sky-400">
                  {Object.values(selectedToImport).filter(Boolean).length}
                </strong>
              </span>

              <div className="flex items-center gap-2">
                <button
                  onClick={() => setShowScanModal(false)}
                  className="px-3.5 py-1.5 rounded-xl border border-white/[0.08] hover:bg-white/[0.04] text-slate-300 text-xs cursor-pointer"
                >
                  Zrušit
                </button>
                <button
                  onClick={handleConfirmImport}
                  className="px-4 py-1.5 rounded-xl bg-sky-500 hover:bg-sky-400 text-slate-950 font-semibold text-xs cursor-pointer shadow-sm"
                >
                  Přidat do mých her
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Manual Add Custom App Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-fadeIn">
          <form
            onSubmit={handleAddCustomApp}
            className={`w-full max-w-md rounded-2xl p-6 border glass-panel shadow-2xl space-y-4 ${
              isDark
                ? 'bg-[#0c0f18] border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <Gamepad2 className="w-5 h-5 text-sky-400" />
              <h3 className="text-base font-bold">Přidat hru nebo aplikaci ručně</h3>
            </div>

            <div className="space-y-3">
              <div>
                <label className="block text-xs font-medium text-slate-400 mb-1">
                  Název aplikace / hry
                </label>
                <input
                  type="text"
                  required
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="např. Cyberpunk 2077"
                  className={`w-full px-3 py-2 text-xs rounded-xl border ${
                    isDark
                      ? 'bg-white/[0.03] border-white/10 text-white'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-400 mb-1">
                  Název spustitelného souboru (.exe)
                </label>
                <input
                  type="text"
                  required
                  value={newExe}
                  onChange={(e) => setNewExe(e.target.value)}
                  placeholder="např. Cyberpunk2077.exe"
                  className={`w-full px-3 py-2 text-xs rounded-xl border font-mono ${
                    isDark
                      ? 'bg-white/[0.03] border-white/10 text-white'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-400 mb-1">
                  Typ HDR
                </label>
                <select
                  value={newType}
                  onChange={(e) => setNewType(e.target.value as HdrType)}
                  className={`w-full px-3 py-2 text-xs rounded-xl border ${
                    isDark
                      ? 'bg-slate-900 border-white/10 text-white'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                >
                  <option value="native">Nativní HDR</option>
                  <option value="autohdr">Windows Auto HDR</option>
                  <option value="custom">Vlastní konfigurace</option>
                </select>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2">
              <button
                type="button"
                onClick={() => setShowAddModal(false)}
                className="px-3.5 py-1.5 rounded-xl border border-white/[0.08] hover:bg-white/[0.04] text-slate-300 text-xs cursor-pointer"
              >
                Zrušit
              </button>
              <button
                type="submit"
                className="px-4 py-1.5 rounded-xl bg-sky-500 hover:bg-sky-400 text-slate-950 font-semibold text-xs cursor-pointer shadow-sm"
              >
                Uložit hru
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};
