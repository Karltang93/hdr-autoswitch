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
      setScanMessage('Chyba při skenování disků.');
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
      setScanMessage(`Úspěšně přidáno ${addedCount} vybraných her do sledování!`);
      setTimeout(() => setScanMessage(null), 5000);
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
      console.error('Failed to add app:', err);
    }
  };

  const filteredApps = config.apps.filter(
    (app) =>
      app.name.toLowerCase().includes(search.toLowerCase()) ||
      app.exe_name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="space-y-5 animate-fadeIn">
      {/* Actions and Search Header */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat v mých nainstalovaných hrách..."
            className={`w-full pl-9 pr-4 py-2 text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-slate-900/60 border-white/10 focus:border-cyan-500 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400'
            }`}
          />
        </div>

        <div className="flex items-center gap-2 flex-wrap sm:flex-nowrap">
          <button
            onClick={handleStartScan}
            disabled={isScanning}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white text-xs font-semibold shadow-md shadow-cyan-600/20 cursor-pointer transition-all disabled:opacity-50"
            title="Prohledá Steam, Epic Games, EA, Ubisoft i registry"
          >
            <ScanSearch className={`w-3.5 h-3.5 ${isScanning ? 'animate-spin' : ''}`} />
            {isScanning ? 'Skenuji PC...' : 'Skenovat nainstalované hry'}
          </button>

          <button
            onClick={() => setShowAddModal(true)}
            className={`flex items-center gap-1.5 px-3 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
              isDark
                ? 'border-white/10 hover:bg-white/5 text-slate-300'
                : 'border-slate-300 hover:bg-slate-100 text-slate-700'
            }`}
          >
            <Plus className="w-4 h-4" /> Přidat ručně
          </button>

          <button
            onClick={onNavigateToCatalog}
            className={`flex items-center gap-1.5 px-3 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
              isDark
                ? 'border-white/10 hover:bg-white/5 text-purple-400'
                : 'border-slate-300 hover:bg-slate-100 text-purple-700'
            }`}
          >
            <Compass className="w-4 h-4" /> Databáze her
          </button>
        </div>
      </div>

      {scanMessage && (
        <div className="p-3 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 text-xs flex items-center gap-2">
          <Sparkles className="w-4 h-4 shrink-0" />
          <span>{scanMessage}</span>
        </div>
      )}

      {/* Installed Apps List */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-slate-900/60 border-white/10' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="divide-y divide-white/5 max-h-[480px] overflow-y-auto">
          {filteredApps.length === 0 ? (
            <div className="p-10 text-center space-y-3">
              <div className="text-slate-500 text-sm font-medium">
                {search
                  ? 'Nenalezena žádná aplikace odpovídající hledání.'
                  : 'Zatím zde nemáte žádné přidané aplikace.'}
              </div>
              {!search && (
                <div className="flex items-center justify-center gap-3">
                  <button
                    onClick={handleStartScan}
                    className="px-4 py-2 rounded-xl text-xs font-semibold bg-cyan-600 hover:bg-cyan-700 text-white cursor-pointer shadow-md"
                  >
                    🔍 Skenovat hry na disku
                  </button>
                  <button
                    onClick={onNavigateToCatalog}
                    className="px-4 py-2 rounded-xl text-xs font-semibold border border-white/10 hover:bg-white/5 text-slate-300 cursor-pointer"
                  >
                    📚 Procházet databázi her
                  </button>
                </div>
              )}
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
                          app.enabled ? '' : 'text-slate-500 line-through'
                        }`}
                      >
                        {app.name}
                      </h4>
                      {app.path && (
                        <span
                          className="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20 font-mono truncate max-w-[250px]"
                          title={app.path}
                        >
                          Nainstalováno
                        </span>
                      )}
                      {app.alternate_exes && app.alternate_exes.length > 0 && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20 font-mono">
                          +{app.alternate_exes.length} procesy
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs text-slate-400 font-mono mt-0.5">
                      <span>{app.exe_name}</span>
                      {app.alternate_exes && app.alternate_exes.length > 0 && (
                        <span className="text-[11px] text-slate-500">
                          (alt: {app.alternate_exes.join(', ')})
                        </span>
                      )}
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
                    <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-500"></div>
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
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-md animate-fadeIn">
          <div
            className={`w-full max-w-lg rounded-2xl p-6 border glass-panel shadow-2xl flex flex-col max-h-[80vh] ${
              isDark
                ? 'bg-slate-900 border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <ScanSearch className="w-5 h-5 text-cyan-400" />
              <h3 className="text-lg font-bold">Nalezené HDR hry v počítači</h3>
            </div>
            <p className="text-xs text-slate-400">
              Prohledali jsme Steam, Epic Games, EA, Ubisoft i systémové registry. Vyberte hry, které chcete automaticky sledovat pro přepínání HDR.
            </p>

            <div className="my-4 divide-y divide-white/5 overflow-y-auto flex-1 pr-1">
              {scannedGames.length === 0 ? (
                <div className="py-8 text-center text-slate-500 text-sm">
                  Nebyly nalezeny žádné podporované hry na discích.
                </div>
              ) : (
                scannedGames.map((game) => {
                  const isChecked = !!selectedToImport[game.exe_name];
                  const alreadyInApps = config.apps.some(
                    (a) =>
                      a.name.toLowerCase() === game.name.toLowerCase() ||
                      a.exe_name.toLowerCase() === game.exe_name.toLowerCase()
                  );

                  return (
                    <div
                      key={game.exe_name}
                      onClick={() =>
                        setSelectedToImport((prev) => ({
                          ...prev,
                          [game.exe_name]: !prev[game.exe_name],
                        }))
                      }
                      className={`p-3 flex items-center justify-between gap-3 cursor-pointer rounded-xl transition-colors ${
                        isDark ? 'hover:bg-white/5' : 'hover:bg-slate-100'
                      }`}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <input
                          type="checkbox"
                          checked={isChecked}
                          onChange={() => {}}
                          className="w-4 h-4 rounded text-cyan-600 focus:ring-cyan-500 cursor-pointer"
                        />
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-semibold text-sm truncate">
                              {game.name}
                            </span>
                            {alreadyInApps && (
                              <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                                Již v mých hrách
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-slate-400 font-mono truncate">
                            {game.exe_name}
                          </p>
                        </div>
                      </div>

                      <Gamepad2 className="w-4 h-4 text-slate-500 shrink-0" />
                    </div>
                  );
                })
              )}
            </div>

            <div className="flex items-center justify-between pt-3 border-t border-white/10">
              <div className="text-xs text-slate-400">
                Vybráno:{' '}
                <strong className="text-cyan-400">
                  {Object.values(selectedToImport).filter(Boolean).length}
                </strong>{' '}
                her
              </div>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShowScanModal(false)}
                  className={`px-3.5 py-2 rounded-xl text-xs font-semibold cursor-pointer border ${
                    isDark
                      ? 'border-white/10 hover:bg-white/5 text-slate-300'
                      : 'border-slate-300 hover:bg-slate-100 text-slate-700'
                  }`}
                >
                  Zavřít
                </button>
                <button
                  type="button"
                  onClick={handleConfirmImport}
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-cyan-600 hover:bg-cyan-700 text-white shadow-md shadow-cyan-600/20 cursor-pointer"
                >
                  Přidat vybrané hry
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

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
