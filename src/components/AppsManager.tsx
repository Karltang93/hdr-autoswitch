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
  LayoutGrid,
  List as ListIcon,
  ShieldCheck,
  Zap,
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
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [selectedLauncher, setSelectedLauncher] = useState<string>('all');
  const [isScanning, setIsScanning] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  // Scan modal
  const [showScanModal, setShowScanModal] = useState(false);
  const [scannedGames, setScannedGames] = useState<HdrApp[]>([]);
  const [selectedToImport, setSelectedToImport] = useState<Record<string, boolean>>({});

  // Manual Add Modal
  const [showAddModal, setShowAddModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newExe, setNewExe] = useState('');
  const [newType, setNewType] = useState<HdrType>('native');

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
      setScanMessage(`Úspěšně přidáno ${addedCount} nových her do sledování!`);
      setTimeout(() => setScanMessage(null), 4500);
    } catch (err) {
      console.error('Failed to import games:', err);
    }
  };

  const handleAddCustomApp = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName.trim() || !newExe.trim()) return;

    let cleanExe = newExe.trim().toLowerCase();
    if (!cleanExe.endsWith('.exe')) cleanExe += '.exe';

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
      setNewType('native');
    } catch (err) {
      console.error('Failed to add custom app:', err);
    }
  };

  const filteredApps = config.apps.filter((app) => {
    const matchesSearch =
      app.name.toLowerCase().includes(search.toLowerCase()) ||
      app.exe_name.toLowerCase().includes(search.toLowerCase());

    const matchesLauncher =
      selectedLauncher === 'all'
        ? true
        : selectedLauncher === 'steam'
        ? !!app.steam_id || app.launcher?.toLowerCase() === 'steam'
        : app.launcher?.toLowerCase() === selectedLauncher.toLowerCase();

    return matchesSearch && matchesLauncher;
  });

  const getHdrBadge = (hdrType: HdrType) => {
    switch (hdrType) {
      case 'native':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-2 py-0.5 rounded-md bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 font-bold uppercase tracking-wider backdrop-blur-md">
            <ShieldCheck className="w-3 h-3" /> Nativní HDR
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-2 py-0.5 rounded-md bg-amber-500/20 text-amber-300 border border-amber-500/40 font-bold uppercase tracking-wider backdrop-blur-md">
            <Zap className="w-3 h-3" /> Auto HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-2 py-0.5 rounded-md bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-bold uppercase tracking-wider backdrop-blur-md">
            Média
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-2 py-0.5 rounded-md bg-purple-500/20 text-purple-300 border border-purple-500/40 font-bold uppercase tracking-wider backdrop-blur-md">
            Vlastní
          </span>
        );
    }
  };

  const steamCount = config.apps.filter((a) => a.steam_id || a.launcher === 'Steam').length;
  const activeCount = config.apps.filter((a) => a.enabled).length;

  return (
    <div className="space-y-5">
      {/* Top Controls Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-black tracking-tight text-white flex items-center gap-2">
            <span>Moje hry</span>
            <span className="text-xs font-mono font-normal px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
              {config.apps.length} celkem • {activeCount} sledováno
            </span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Aktivujte nebo pozastavte automatické přepínání HDR pro jednotlivé hry.
          </p>
        </div>

        {/* Action Buttons with High-Energy Neon Polish */}
        <div className="flex items-center gap-2.5 flex-wrap">
          <button
            onClick={handleStartScan}
            disabled={isScanning}
            className="flex items-center gap-2 px-4 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white font-bold text-xs shadow-md neon-glow-cyan cursor-pointer transition-all duration-150 disabled:opacity-50"
          >
            <ScanSearch className={`w-4 h-4 ${isScanning ? 'animate-spin' : ''}`} />
            <span>{isScanning ? 'Prohledávám disky...' : 'Skenovat hry v PC'}</span>
          </button>

          <button
            onClick={() => setShowAddModal(true)}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
              isDark
                ? 'border-white/10 hover:border-cyan-500/40 hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-xs'
            }`}
          >
            <Plus className="w-3.5 h-3.5 text-cyan-400" />
            <span>Přidat ručně</span>
          </button>

          <button
            onClick={onNavigateToCatalog}
            className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
              isDark
                ? 'border-white/10 hover:border-purple-500/40 hover:bg-white/[0.04] text-slate-300'
                : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-xs'
            }`}
          >
            <Compass className="w-3.5 h-3.5 text-purple-400" />
            <span>Katalog</span>
          </button>
        </div>
      </div>

      {scanMessage && (
        <div className="p-3.5 rounded-2xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-300 text-xs flex items-center gap-2.5 shadow-sm">
          <Sparkles className="w-4 h-4 shrink-0 text-cyan-400" />
          <span className="font-medium">{scanMessage}</span>
        </div>
      )}

      {/* Filter and View Mode Toolbar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        {/* Search input */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat mezi nainstalovanými hrami..."
            className={`w-full pl-9 pr-4 py-2 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0e1322]/80 border-white/[0.08] focus:border-cyan-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400 shadow-xs'
            }`}
          />
        </div>

        {/* View mode toggle (Grid vs List) */}
        <div className="flex items-center gap-1.5 bg-white/[0.03] p-1 rounded-xl border border-white/[0.08]">
          <button
            onClick={() => setViewMode('grid')}
            className={`p-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-all ${
              viewMode === 'grid'
                ? 'bg-cyan-500 text-slate-950 shadow-sm'
                : 'text-slate-400 hover:text-white'
            }`}
            title="Zobrazení mřížky plakátů (Grid)"
          >
            <LayoutGrid className="w-4 h-4" />
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={`p-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-all ${
              viewMode === 'list'
                ? 'bg-cyan-500 text-slate-950 shadow-sm'
                : 'text-slate-400 hover:text-white'
            }`}
            title="Zobrazení řádků (Seznam)"
          >
            <ListIcon className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Launcher Filter Tabs */}
      <div className="flex items-center gap-2 overflow-x-auto pb-1">
        {[
          { id: 'all', label: `Všechny (${config.apps.length})` },
          { id: 'steam', label: `Steam (${steamCount})` },
          { id: 'epic games', label: 'Epic Games' },
          { id: 'windows', label: 'Windows / Ostatní' },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setSelectedLauncher(tab.id)}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all ${
              selectedLauncher === tab.id
                ? 'bg-cyan-500 text-slate-950 font-bold shadow-sm neon-glow-cyan'
                : 'bg-white/[0.03] text-slate-400 hover:text-white border border-white/[0.06] hover:border-cyan-500/30'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Main Content: Posters Grid or List */}
      {filteredApps.length === 0 ? (
        <div className="p-16 text-center rounded-3xl border border-white/[0.06] glass-panel bg-[#0e1322]/50 space-y-4">
          <Gamepad2 className="w-12 h-12 text-slate-600 mx-auto stroke-1" />
          <div className="space-y-1">
            <h3 className="text-base font-bold text-slate-300">
              {search
                ? 'Nenalezena žádná hra odpovídající hledání'
                : 'Zatím zde nemáte přidané žádné hry'}
            </h3>
            <p className="text-xs text-slate-500 max-w-md mx-auto">
              Prohledejte disky pro automatické nalezení nainstalovaných her nebo projděte databázi podporovaných titulů.
            </p>
          </div>
          {!search && (
            <div className="flex items-center justify-center gap-3 pt-2">
              <button
                onClick={handleStartScan}
                className="px-4 py-2 rounded-xl text-xs font-bold bg-cyan-500 hover:bg-cyan-400 text-slate-950 cursor-pointer shadow-md neon-glow-cyan"
              >
                Skenovat hry v PC
              </button>
              <button
                onClick={onNavigateToCatalog}
                className="px-4 py-2 rounded-xl text-xs font-semibold border border-white/10 hover:bg-white/5 text-slate-300 cursor-pointer"
              >
                Procházet katalog
              </button>
            </div>
          )}
        </div>
      ) : viewMode === 'grid' ? (
        /* Poster Cards Grid (Like DLSS Swapper) */
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
          {filteredApps.map((app) => {
            const steamCover = app.steam_id
              ? `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${app.steam_id}/library_600x900.jpg`
              : null;

            return (
              <div
                key={app.exe_name}
                className={`group relative rounded-2xl overflow-hidden border transition-all duration-200 hover-card-lift glass-panel poster-card flex flex-col justify-between ${
                  app.enabled
                    ? isDark
                      ? 'bg-[#101524]/85 border-white/[0.08] hover:border-cyan-500/50 hover:shadow-cyan-500/20'
                      : 'bg-white border-slate-200 shadow-md'
                    : 'opacity-55 grayscale hover:grayscale-0 bg-slate-900/50 border-white/[0.04]'
                }`}
              >
                {/* Poster Cover Image */}
                {steamCover ? (
                  <img
                    src={steamCover}
                    alt={app.name}
                    className="absolute inset-0 w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                    onError={(e) => {
                      (e.target as HTMLElement).style.display = 'none';
                    }}
                  />
                ) : (
                  <div className="absolute inset-0 bg-gradient-to-br from-[#12192e] via-[#0d1222] to-[#1a152e] flex items-center justify-center p-4">
                    <Gamepad2 className="w-12 h-12 text-slate-700/60 stroke-1" />
                  </div>
                )}

                {/* Dark Vignette Overlay for Crisp Readability */}
                <div className="absolute inset-0 bg-gradient-to-t from-black/95 via-black/35 to-black/60 pointer-events-none" />

                {/* Top Badges (HDR Tier & Platform) */}
                <div className="relative z-10 p-2.5 flex items-center justify-between gap-1">
                  <div>{getHdrBadge(app.hdr_type)}</div>

                  {app.launcher && (
                    <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-black/60 text-slate-300 backdrop-blur-md border border-white/10">
                      {app.launcher}
                    </span>
                  )}
                </div>

                {/* Bottom Card Content & Controls */}
                <div className="relative z-10 p-3 space-y-2">
                  <div>
                    <h4 className="font-extrabold text-xs text-white drop-shadow-md truncate" title={app.name}>
                      {app.name}
                    </h4>
                    <div className="flex items-center gap-1.5 text-[10px] text-slate-400 font-mono mt-0.5">
                      <span className="truncate max-w-[120px]">{app.exe_name}</span>
                      {app.alternate_exes && app.alternate_exes.length > 0 && (
                        <span
                          className="px-1 py-0.2 rounded bg-purple-500/20 text-purple-300 cursor-help"
                          title={`Alternativní procesy:\n${app.alternate_exes.join('\n')}`}
                        >
                          +{app.alternate_exes.length}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Toggle Switch and Delete Button Row */}
                  <div className="flex items-center justify-between pt-1 border-t border-white/10">
                    <button
                      onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                      className={`inline-flex items-center gap-1.5 text-[10px] font-bold px-2 py-0.5 rounded-full cursor-pointer transition-all ${
                        app.enabled
                          ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40'
                          : 'bg-slate-800 text-slate-400 border border-white/5'
                      }`}
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${
                          app.enabled ? 'bg-cyan-400' : 'bg-slate-500'
                        }`}
                      />
                      <span>{app.enabled ? 'Sledováno' : 'Vypnuto'}</span>
                    </button>

                    <button
                      onClick={() => handleDeleteApp(app.exe_name)}
                      className="p-1 rounded-md text-slate-400 hover:text-rose-400 hover:bg-rose-500/20 cursor-pointer transition-colors"
                      title="Odebrat z mých her"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        /* List View (Dense Table Alternative) */
        <div
          className={`rounded-2xl border overflow-hidden glass-panel ${
            isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
          }`}
        >
          <div className="divide-y divide-white/[0.05] max-h-[520px] overflow-y-auto">
            {filteredApps.map((app) => (
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
                  >
                    {app.enabled ? (
                      <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                    ) : (
                      <XCircle className="w-5 h-5 text-slate-500" />
                    )}
                  </button>

                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4
                        className={`text-sm font-bold truncate ${
                          app.enabled ? 'text-slate-100' : 'text-slate-500 line-through'
                        }`}
                      >
                        {app.name}
                      </h4>
                      {getHdrBadge(app.hdr_type)}
                      {app.launcher && (
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-slate-800 text-slate-400 font-mono">
                          {app.launcher}
                        </span>
                      )}
                      {app.alternate_exes && app.alternate_exes.length > 0 && (
                        <span
                          className="text-[10px] px-1.5 py-0.2 rounded bg-purple-500/10 text-purple-300 font-mono cursor-help"
                          title={`Alternativní procesy:\n${app.alternate_exes.join('\n')}`}
                        >
                          +{app.alternate_exes.length} procesů
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-slate-400 font-mono mt-0.5">
                      {app.exe_name}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3 shrink-0">
                  <button
                    onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                    className={`text-xs font-bold px-3 py-1 rounded-lg border transition-all cursor-pointer ${
                      app.enabled
                        ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 hover:bg-cyan-500/30'
                        : 'bg-slate-800 text-slate-400 border-white/5 hover:bg-slate-700'
                    }`}
                  >
                    {app.enabled ? 'Aktivní' : 'Vypnuto'}
                  </button>

                  <button
                    onClick={() => handleDeleteApp(app.exe_name)}
                    className="p-1.5 rounded-lg text-slate-400 hover:text-rose-400 hover:bg-rose-500/15 cursor-pointer transition-colors"
                    title="Odebrat z mých her"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Scan Results Modal */}
      {showScanModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fadeIn">
          <div
            className={`w-full max-w-xl rounded-3xl p-6 border glass-panel shadow-2xl flex flex-col max-h-[85vh] ${
              isDark
                ? 'bg-[#0f1422] border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <div className="flex items-center gap-2.5 mb-1">
              <ScanSearch className="w-6 h-6 text-cyan-400" />
              <h3 className="text-lg font-extrabold">Nalezené HDR hry v počítači</h3>
            </div>
            <p className="text-xs text-slate-400 mb-4">
              Nalezené tituly z knihoven Steam, Epic Games, EA i registrů. Vyberte, které chcete automaticky sledovat.
            </p>

            <div className="my-2 divide-y divide-white/[0.04] overflow-y-auto flex-1 pr-1 border rounded-2xl border-white/[0.08] p-2 bg-black/20">
              {scannedGames.length === 0 ? (
                <div className="py-12 text-center text-slate-500 text-sm">
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
                      className={`flex items-center justify-between p-3 rounded-xl cursor-pointer transition-colors ${
                        alreadyInApps
                          ? 'opacity-50 bg-white/[0.01]'
                          : isDark
                          ? 'hover:bg-white/[0.04]'
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
                          className="w-4 h-4 rounded text-cyan-500 border-slate-600 focus:ring-cyan-500 cursor-pointer"
                        />
                        <div className="min-w-0">
                          <div className="font-bold text-xs truncate flex items-center gap-2">
                            <span>{game.name}</span>
                            {alreadyInApps && (
                              <span className="text-[10px] text-emerald-400 font-semibold">
                                (již přidáno)
                              </span>
                            )}
                          </div>
                          <div className="text-[11px] text-slate-400 font-mono">
                            {game.exe_name}
                          </div>
                        </div>
                      </div>

                      <div className="shrink-0">{getHdrBadge(game.hdr_type)}</div>
                    </label>
                  );
                })
              )}
            </div>

            <div className="flex items-center justify-between mt-4 pt-3 border-t border-white/[0.08]">
              <span className="text-xs text-slate-400">
                Vybráno k přidání:{' '}
                <strong className="text-cyan-400 font-bold">
                  {Object.values(selectedToImport).filter(Boolean).length}
                </strong>
              </span>

              <div className="flex items-center gap-2.5">
                <button
                  onClick={() => setShowScanModal(false)}
                  className="px-4 py-2 rounded-xl border border-white/10 hover:bg-white/5 text-slate-300 text-xs font-semibold cursor-pointer"
                >
                  Zrušit
                </button>
                <button
                  onClick={handleConfirmImport}
                  className="px-5 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white font-bold text-xs shadow-md neon-glow-cyan cursor-pointer"
                >
                  Přidat vybrané hry
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Manual Add Custom App Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fadeIn">
          <form
            onSubmit={handleAddCustomApp}
            className={`w-full max-w-md rounded-3xl p-6 border glass-panel shadow-2xl space-y-4 ${
              isDark
                ? 'bg-[#0f1422] border-white/10 text-white'
                : 'bg-white border-slate-200 text-slate-900'
            }`}
          >
            <div className="flex items-center gap-2.5">
              <Gamepad2 className="w-6 h-6 text-cyan-400" />
              <h3 className="text-lg font-extrabold">Přidat hru nebo aplikaci ručně</h3>
            </div>

            <div className="space-y-3.5">
              <div>
                <label className="block text-xs font-semibold text-slate-300 mb-1">
                  Název aplikace / hry
                </label>
                <input
                  type="text"
                  required
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="např. Cyberpunk 2077"
                  className={`w-full px-3.5 py-2 text-xs rounded-xl border ${
                    isDark
                      ? 'bg-white/[0.04] border-white/10 text-white focus:border-cyan-500'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 mb-1">
                  Název spustitelného souboru (.exe)
                </label>
                <input
                  type="text"
                  required
                  value={newExe}
                  onChange={(e) => setNewExe(e.target.value)}
                  placeholder="např. Cyberpunk2077.exe"
                  className={`w-full px-3.5 py-2 text-xs rounded-xl border font-mono ${
                    isDark
                      ? 'bg-white/[0.04] border-white/10 text-white focus:border-cyan-500'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-300 mb-1">
                  Typ HDR podpory
                </label>
                <select
                  value={newType}
                  onChange={(e) => setNewType(e.target.value as HdrType)}
                  className={`w-full px-3.5 py-2 text-xs rounded-xl border ${
                    isDark
                      ? 'bg-slate-900 border-white/10 text-white'
                      : 'bg-white border-slate-300 text-slate-900'
                  }`}
                >
                  <option value="native">Nativní HDR</option>
                  <option value="autohdr">Windows Auto HDR</option>
                  <option value="custom">Vlastní konfigurace</option>
                  <option value="media">Přehrávač médií</option>
                </select>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-2">
              <button
                type="button"
                onClick={() => setShowAddModal(false)}
                className="px-4 py-2 rounded-xl border border-white/10 hover:bg-white/5 text-slate-300 text-xs font-semibold cursor-pointer"
              >
                Zrušit
              </button>
              <button
                type="submit"
                className="px-5 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white font-bold text-xs shadow-md neon-glow-cyan cursor-pointer"
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
