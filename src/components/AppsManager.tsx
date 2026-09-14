import React, { useState } from 'react';
import { HdrApp, HdrType, AppConfig } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Search,
  Plus,
  Compass,
  Trash2,
  ScanSearch,
  Sparkles,
  Gamepad2,
  LayoutGrid,
  List as ListIcon,
  ShieldCheck,
  Zap,
  Check,
  X,
} from 'lucide-react';
import { GlitchButton } from './GlitchButton';
import { GlitchText } from './GlitchText';

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
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-cyan-950/90 text-[#5accf5] border border-[#5accf5]/50 font-bold uppercase tracking-wider">
            <ShieldCheck className="w-3 h-3 text-[#5accf5]" /> NATIVNÍ HDR
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-purple-950/90 text-purple-300 border border-purple-500/50 font-bold uppercase tracking-wider">
            <Zap className="w-3 h-3 text-purple-400" /> AUTO HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-blue-950/90 text-blue-300 border border-blue-500/50 font-bold uppercase tracking-wider">
            MÉDIA
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-amber-950/90 text-amber-300 border border-amber-500/50 font-bold uppercase tracking-wider">
            VLASTNÍ / MOD
          </span>
        );
    }
  };

  const steamCount = config.apps.filter((a) => a.steam_id || a.launcher === 'Steam').length;
  const activeCount = config.apps.filter((a) => a.enabled).length;

  return (
    <div className="space-y-5 font-mono">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2.5">
            <h2 className="glitch-title-bar px-2.5 py-0.5 text-xs font-bold tracking-wider inline-block">
              MOJE KNIHOVNA HER
            </h2>
            <span className="text-xs px-2 py-0.5 border border-[#5accf5]/40 text-[#5accf5] bg-[#140e10]">
              {config.apps.length} CELKEM • {activeCount} SLEDOVÁNO
            </span>
          </div>
          <p className="text-xs text-[#8a7f81] mt-1">
            Hry v tomto seznamu automaticky přepnou displej do HDR režimu při zaměření okna.
          </p>
        </div>

        {/* Top Action Buttons with CodePen Glitch styling */}
        <div className="flex items-center gap-2.5 flex-wrap">
          <GlitchButton
            label={isScanning ? 'PROHLEDÁVÁM...' : 'SKENOVAT HRY V PC'}
            variant="primary"
            size="sm"
            disabled={isScanning}
            icon={<ScanSearch className={`w-3.5 h-3.5 ${isScanning ? 'animate-spin' : ''}`} />}
            onClick={handleStartScan}
          />

          <GlitchButton
            label="PŘIDAT RUČNĚ"
            variant="outline"
            size="sm"
            icon={<Plus className="w-3.5 h-3.5 text-[#5accf5]" />}
            onClick={() => setShowAddModal(true)}
          />

          <GlitchButton
            label="KATALOG"
            variant="outline"
            size="sm"
            icon={<Compass className="w-3.5 h-3.5 text-[#f55a6b]" />}
            onClick={onNavigateToCatalog}
          />
        </div>
      </div>

      {scanMessage && (
        <div className="p-3 border border-[#5accf5]/40 bg-[#120e10] text-[#5accf5] text-xs flex items-center gap-2.5">
          <Sparkles className="w-4 h-4 shrink-0 text-[#5accf5]" />
          <span>&gt; {scanMessage}</span>
        </div>
      )}

      {/* Filter and View Mode Toolbar */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        {/* Search input */}
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8a7f81]" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat mezi nainstalovanými hrami..."
            className="w-full pl-9 pr-4 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white placeholder-[#8a7f81] focus:outline-none transition-all"
          />
        </div>

        {/* View mode toggle (Grid vs List) */}
        <div className="flex items-center gap-1 bg-[#120d0e] p-1 border border-[#f55a6b]/30">
          <button
            onClick={() => setViewMode('grid')}
            className={`p-1 text-xs cursor-pointer transition-all ${
              viewMode === 'grid'
                ? 'bg-[#f55a6b] text-[#0f0b0b]'
                : 'text-[#8a7f81] hover:text-white'
            }`}
            title="Zobrazení plakátů (Grid)"
          >
            <LayoutGrid className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={`p-1 text-xs cursor-pointer transition-all ${
              viewMode === 'list'
                ? 'bg-[#f55a6b] text-[#0f0b0b]'
                : 'text-[#8a7f81] hover:text-white'
            }`}
            title="Zobrazení řádků (Seznam)"
          >
            <ListIcon className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Launcher Filter Tabs */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
        {[
          { id: 'all', label: `VŠECHNY (${config.apps.length})` },
          { id: 'steam', label: `STEAM (${steamCount})` },
          { id: 'epic games', label: 'EPIC GAMES' },
          { id: 'windows', label: 'WINDOWS / OSTATNÍ' },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setSelectedLauncher(tab.id)}
            className={`px-3 py-1 text-xs uppercase font-bold cursor-pointer transition-all border ${
              selectedLauncher === tab.id
                ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                : 'bg-[#120d0e] text-[#8a7f81] border-[#f55a6b]/20 hover:border-[#f55a6b]/50 hover:text-white'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Main Content: Posters Grid or List */}
      {filteredApps.length === 0 ? (
        <div className="p-16 text-center border border-[#f55a6b]/20 bg-[#120d0e] space-y-4">
          <Gamepad2 className="w-12 h-12 text-[#8a7f81] mx-auto stroke-1" />
          <div className="space-y-1">
            <h3 className="text-sm font-bold text-white">
              {search
                ? '&gt; Žádná hra neodpovídá hledání.'
                : '&gt; Zatím zde nemáte žádné sledované hry.'}
            </h3>
            <p className="text-xs text-[#8a7f81] max-w-md mx-auto">
              Spusťte skenování disků pro automatické nalezení her nebo si vyberte z databáze PCGamingWiki.
            </p>
          </div>
          {!search && (
            <div className="flex items-center justify-center gap-3 pt-2">
              <GlitchButton
                label="SKENOVAT DISKY"
                variant="primary"
                size="sm"
                onClick={handleStartScan}
              />
              <GlitchButton
                label="PROCHÁZET KATALOG"
                variant="outline"
                size="sm"
                onClick={onNavigateToCatalog}
              />
            </div>
          )}
        </div>
      ) : viewMode === 'grid' ? (
        /* Poster Cards Grid (2:3 aspect ratio) */
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3.5">
          {filteredApps.map((app) => {
            const steamCover = app.steam_id
              ? `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${app.steam_id}/library_600x900.jpg`
              : null;

            return (
              <div
                key={app.exe_name}
                className={`relative group border overflow-hidden flex flex-col justify-between transition-all duration-200 ${
                  app.enabled
                    ? 'bg-[#120d0e] border-[#f55a6b]/35 hover:border-[#f55a6b] hover:shadow-[0_0_15px_rgba(245,90,107,0.3)]'
                    : 'bg-[#120d0e]/60 border-white/10 opacity-65'
                }`}
                style={{ height: '240px' }}
              >
                {/* Poster Artwork with Scanlines */}
                <div className="absolute inset-0">
                  {steamCover ? (
                    <img
                      src={steamCover}
                      alt={app.name}
                      className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                      onError={(e) => {
                        (e.target as HTMLElement).style.display = 'none';
                      }}
                    />
                  ) : (
                    <div className="w-full h-full bg-gradient-to-b from-[#221314] to-[#0f0b0b]" />
                  )}
                  {/* CRT Scanline overlay on image */}
                  <div className="absolute inset-0 scanlines-overlay opacity-35 pointer-events-none" />
                  <div className="absolute inset-0 bg-gradient-to-t from-[#0f0b0b] via-[#0f0b0b]/60 to-transparent pointer-events-none" />
                </div>

                {/* Top Badges */}
                <div className="relative z-10 p-2 flex items-center justify-between">
                  {getHdrBadge(app.hdr_type)}
                  {app.launcher && (
                    <span className="px-1 py-0.2 text-[8px] font-mono text-[#b5a9ac] bg-black/60 border border-white/10 uppercase">
                      {app.launcher}
                    </span>
                  )}
                </div>

                {/* Bottom Overlay & Controls */}
                <div className="relative z-10 p-2.5 space-y-1.5 bg-[#0f0b0b]/90 border-t border-[#f55a6b]/20">
                  <div className="font-bold text-xs truncate text-white" title={app.name}>
                    <GlitchText text={app.name} scrambleOnHover={true} />
                  </div>

                  <div className="text-[10px] text-[#5accf5] font-mono truncate">
                    [{app.exe_name}]
                  </div>

                  <div className="flex items-center justify-between pt-0.5">
                    {/* Toggle button */}
                    <button
                      onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                      className={`px-2 py-0.5 text-[9px] font-bold uppercase tracking-wider cursor-pointer border transition-all ${
                        app.enabled
                          ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                          : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30 hover:border-white'
                      }`}
                    >
                      {app.enabled ? 'SLEDOVÁNO' : 'POZASTAVENO'}
                    </button>

                    {/* Delete button */}
                    <button
                      onClick={() => handleDeleteApp(app.exe_name)}
                      className="p-1 text-[#8a7f81] hover:text-[#f55a6b] cursor-pointer transition-colors"
                      title="Odebrat z knihovny"
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
        /* List Mode View */
        <div className="space-y-2">
          {filteredApps.map((app) => {
            const steamCover = app.steam_id
              ? `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${app.steam_id}/capsule_sm_120.jpg`
              : null;

            return (
              <div
                key={app.exe_name}
                className={`p-3 border transition-all flex items-center justify-between gap-4 relative ${
                  app.enabled
                    ? 'bg-[#120d0e] border-[#f55a6b]/35 hover:border-[#f55a6b]'
                    : 'bg-[#120d0e]/50 border-white/10 opacity-65'
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  {/* Thumbnail */}
                  <div className="w-16 h-10 border border-[#f55a6b]/30 bg-black shrink-0 overflow-hidden relative">
                    {steamCover ? (
                      <img src={steamCover} alt={app.name} className="w-full h-full object-cover" />
                    ) : (
                      <div className="w-full h-full bg-[#221314] flex items-center justify-center">
                        <Gamepad2 className="w-4 h-4 text-[#8a7f81]" />
                      </div>
                    )}
                    <div className="absolute inset-0 scanlines-overlay opacity-20" />
                  </div>

                  <div className="space-y-0.5 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-bold text-sm text-white truncate">{app.name}</span>
                      {getHdrBadge(app.hdr_type)}
                      {app.launcher && (
                        <span className="text-[9px] px-1.5 py-0.2 bg-black border border-white/15 text-[#b5a9ac] uppercase">
                          {app.launcher}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-[#5accf5] font-mono">[{app.exe_name}]</div>
                  </div>
                </div>

                <div className="flex items-center gap-3 shrink-0">
                  <button
                    onClick={() => handleToggleApp(app.exe_name, !app.enabled)}
                    className={`px-2.5 py-1 text-xs font-bold uppercase tracking-wider cursor-pointer border transition-all ${
                      app.enabled
                        ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]'
                        : 'bg-[#120d0e] text-[#8a7f81] border-[#8a7f81]/30'
                    }`}
                  >
                    {app.enabled ? 'SLEDOVÁNO' : 'POZASTAVENO'}
                  </button>

                  <button
                    onClick={() => handleDeleteApp(app.exe_name)}
                    className="p-1.5 text-[#8a7f81] hover:text-[#f55a6b] cursor-pointer transition-colors"
                    title="Odebrat hru"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Scan Modal */}
      {showScanModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
          <div className="bg-[#0f0b0b] border-2 border-[#f55a6b] max-w-xl w-full p-6 space-y-4 relative shadow-[0_0_30px_rgba(245,90,107,0.4)]">
            <div className="flex items-center justify-between border-b border-[#f55a6b]/30 pb-3">
              <div className="flex items-center gap-2">
                <ScanSearch className="w-5 h-5 text-[#5accf5]" />
                <h3 className="glitch-title-bar px-2 py-0.5 text-xs font-bold uppercase">
                  NALEZENÉ HRY V PC ({scannedGames.length})
                </h3>
              </div>
              <button
                onClick={() => setShowScanModal(false)}
                className="text-[#8a7f81] hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <p className="text-xs text-[#8a7f81]">
              Vyberte hry, které chcete přidat do automatického sledování HDR:
            </p>

            <div className="max-h-80 overflow-y-auto space-y-1.5 border border-[#f55a6b]/20 p-2 bg-[#120d0e]">
              {scannedGames.map((game) => {
                const isSelected = !!selectedToImport[game.exe_name];
                return (
                  <div
                    key={game.exe_name}
                    onClick={() =>
                      setSelectedToImport((prev) => ({
                        ...prev,
                        [game.exe_name]: !prev[game.exe_name],
                      }))
                    }
                    className={`p-2.5 border cursor-pointer flex items-center justify-between text-xs transition-all ${
                      isSelected
                        ? 'bg-[#1c0f12] border-[#f55a6b] text-white'
                        : 'bg-black/40 border-white/10 text-[#8a7f81] hover:border-white/30'
                    }`}
                  >
                    <div className="space-y-0.5">
                      <div className="font-bold flex items-center gap-2">
                        <span>{game.name}</span>
                        {game.launcher && (
                          <span className="text-[9px] px-1 bg-black border border-white/20 text-[#5accf5]">
                            {game.launcher}
                          </span>
                        )}
                      </div>
                      <div className="text-[10px] text-[#5accf5] font-mono">[{game.exe_name}]</div>
                    </div>

                    <div
                      className={`w-4 h-4 border flex items-center justify-center ${
                        isSelected
                          ? 'bg-[#f55a6b] border-[#f55a6b] text-black'
                          : 'border-[#8a7f81]'
                      }`}
                    >
                      {isSelected && <Check className="w-3 h-3 stroke-[3]" />}
                    </div>
                  </div>
                );
              })}
            </div>

            <div className="flex items-center justify-end gap-3 pt-2">
              <GlitchButton
                label="ZRUŠIT"
                variant="outline"
                size="sm"
                onClick={() => setShowScanModal(false)}
              />
              <GlitchButton
                label="PŘIDAT VYBRANÉ"
                variant="primary"
                size="sm"
                onClick={handleConfirmImport}
              />
            </div>
          </div>
        </div>
      )}

      {/* Manual Add Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
          <div className="bg-[#0f0b0b] border-2 border-[#f55a6b] max-w-md w-full p-6 space-y-4 relative shadow-[0_0_30px_rgba(245,90,107,0.4)]">
            <div className="flex items-center justify-between border-b border-[#f55a6b]/30 pb-3">
              <h3 className="glitch-title-bar px-2 py-0.5 text-xs font-bold uppercase">
                PŘIDAT HRU RUČNĚ
              </h3>
              <button
                onClick={() => setShowAddModal(false)}
                className="text-[#8a7f81] hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleAddCustomApp} className="space-y-4">
              <div className="space-y-1">
                <label className="text-xs uppercase text-[#8a7f81]">NÁZEV HRY</label>
                <input
                  type="text"
                  required
                  placeholder="např. Cyberpunk 2077"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none"
                />
              </div>

              <div className="space-y-1">
                <label className="text-xs uppercase text-[#8a7f81]">NÁZEV EXEKUTIVY (.EXE)</label>
                <input
                  type="text"
                  required
                  placeholder="např. cyberpunk2077.exe"
                  value={newExe}
                  onChange={(e) => setNewExe(e.target.value)}
                  className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none"
                />
              </div>

              <div className="space-y-1">
                <label className="text-xs uppercase text-[#8a7f81]">TYP HDR PODPORY</label>
                <select
                  value={newType}
                  onChange={(e) => setNewType(e.target.value as HdrType)}
                  className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none"
                >
                  <option value="native">Nativní HDR10</option>
                  <option value="autohdr">Windows Auto HDR</option>
                  <option value="custom">Vlastní / Mod</option>
                  <option value="media">Média / Video přehrávač</option>
                </select>
              </div>

              <div className="flex items-center justify-end gap-3 pt-3">
                <GlitchButton
                  type="button"
                  label="ZRUŠIT"
                  variant="outline"
                  size="sm"
                  onClick={() => setShowAddModal(false)}
                />
                <GlitchButton
                  type="submit"
                  label="PŘIDAT HRU"
                  variant="primary"
                  size="sm"
                />
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
