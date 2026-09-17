import React, { useState, useEffect } from 'react';
import { HdrApp, HdrType, AppConfig, PickedGameInfo } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
  FolderOpen,
  UploadCloud,
  AlertCircle,
  RefreshCw,
} from 'lucide-react';
import { GlitchButton } from './GlitchButton';
import { GlitchText } from './GlitchText';
import { useI18n } from '../i18n';

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
  const { t } = useI18n();
  const [search, setSearch] = useState('');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [selectedLauncher, setSelectedLauncher] = useState<string>('all');
  const [isScanning, setIsScanning] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  // Scan modal
  const [showScanModal, setShowScanModal] = useState(false);
  const [scannedGames, setScannedGames] = useState<HdrApp[]>([]);
  const [selectedToImport, setSelectedToImport] = useState<Record<string, boolean>>({});
  const [pathStatus, setPathStatus] = useState<Record<string, boolean>>({});

  // Verify paths of apps currently in the user's library
  useEffect(() => {
    const paths = config.apps
      .map((a) => a.path)
      .filter((p): p is string => !!p && p.trim().length > 0);

    if (paths.length > 0) {
      invoke<Record<string, boolean>>('verify_game_paths', { paths })
        .then((status) => setPathStatus(status))
        .catch((err) => console.error('Failed to verify app paths:', err));
    }
  }, [config.apps]);

  const findExistingApp = (item: HdrApp): HdrApp | undefined => {
    const itemExe = item.exe_name.toLowerCase();
    const itemName = item.name.toLowerCase();
    return config.apps.find((a) => {
      const aExe = a.exe_name.toLowerCase();
      const aName = a.name.toLowerCase();
      const matchesExe =
        aExe === itemExe ||
        (a.alternate_exes && a.alternate_exes.some((x) => x.toLowerCase() === itemExe));
      const matchesName = aName === itemName;
      return matchesExe || matchesName;
    });
  };

  const isPathDifferent = (existing: HdrApp, scanned: HdrApp): boolean => {
    if (!scanned.path) return false;
    if (!existing.path) return true; // Path missing previously, now found on disk!
    const normOld = existing.path.replace(/\//g, '\\').toLowerCase().trim();
    const normNew = scanned.path.replace(/\//g, '\\').toLowerCase().trim();
    return normOld !== normNew;
  };

  // Manual Add Modal & File Picker
  const [showAddModal, setShowAddModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newExe, setNewExe] = useState('');
  const [newPath, setNewPath] = useState('');
  const [newType, setNewType] = useState<HdrType>('native');
  const [isHdrMatched, setIsHdrMatched] = useState(false);
  const [isDraggingOver, setIsDraggingOver] = useState(false);

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
        const existing = findExistingApp(item);
        if (existing) {
          const pathChanged = isPathDifferent(existing, item);
          // If game is already tracked:
          // - If path moved or was newly discovered: PRE-SELECT to update path!
          // - If already up to date: uncheck by default
          initialSelected[item.exe_name] = pathChanged;
        } else {
          // New game: pre-select HDR games, leave SDR unselected by default
          initialSelected[item.exe_name] = item.enabled;
        }
      }
      setSelectedToImport(initialSelected);
      setShowScanModal(true);
    } catch (err) {
      console.error('Failed to scan installed games:', err);
      setScanMessage(t.scanModalError);
    } finally {
      setIsScanning(false);
    }
  };

  const selectAllHdr = () => {
    const next: Record<string, boolean> = {};
    for (const item of scannedGames) {
      if (item.enabled) {
        next[item.exe_name] = true;
      }
    }
    setSelectedToImport(next);
  };

  const selectAll = () => {
    const next: Record<string, boolean> = {};
    for (const item of scannedGames) {
      next[item.exe_name] = true;
    }
    setSelectedToImport(next);
  };

  const deselectAll = () => {
    setSelectedToImport({});
  };

  const handleBrowseExe = async () => {
    try {
      const picked: PickedGameInfo | null = await invoke('pick_game_exe');
      if (picked) {
        setNewName(picked.name);
        setNewExe(picked.exe_name);
        setNewPath(picked.path);
        setNewType(picked.hdr_type.toLowerCase() as HdrType);
        setIsHdrMatched(picked.is_hdr_supported);
        setShowAddModal(true);
      }
    } catch (err) {
      console.error('Failed to pick game exe:', err);
    }
  };

  useEffect(() => {
    const unlistenPromise = listen<{ paths?: string[] }>('tauri://drag-drop', async (event) => {
      const paths = event.payload?.paths;
      if (paths && paths.length > 0) {
        const exePath = paths.find((p) => p.toLowerCase().endsWith('.exe'));
        if (exePath) {
          try {
            const picked: PickedGameInfo = await invoke('inspect_exe_path', { path: exePath });
            setNewName(picked.name);
            setNewExe(picked.exe_name);
            setNewPath(picked.path);
            setNewType(picked.hdr_type.toLowerCase() as HdrType);
            setIsHdrMatched(picked.is_hdr_supported);
            setShowAddModal(true);
          } catch (err) {
            console.error('Failed to inspect dropped exe:', err);
          }
        }
      }
    });

    return () => {
      unlistenPromise.then((un) => un());
    };
  }, []);

  const handleConfirmImport = async () => {
    const toImport = scannedGames
      .filter((g) => selectedToImport[g.exe_name])
      .map((g) => ({
        ...g,
        enabled: true, // Imported games that the user selected are enabled
      }));

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
      setScanMessage(t.scanModalSuccess(addedCount));
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
      path: newPath || undefined,
    };

    try {
      await invoke('add_custom_app', { app: newApp });
      const refreshedConfig: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshedConfig);
      setShowAddModal(false);
      setNewName('');
      setNewExe('');
      setNewPath('');
      setIsHdrMatched(false);
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
            <ShieldCheck className="w-3 h-3 text-[#5accf5]" /> {t.recentTierNative}
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-purple-950/90 text-purple-300 border border-purple-500/50 font-bold uppercase tracking-wider">
            <Zap className="w-3 h-3 text-purple-400" /> {t.recentTierAutoHdr}
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-blue-950/90 text-blue-300 border border-blue-500/50 font-bold uppercase tracking-wider">
            {t.catalogTierMedia.toUpperCase()}
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 text-[9px] font-mono px-1.5 py-0.5 bg-amber-950/90 text-amber-300 border border-amber-500/50 font-bold uppercase tracking-wider">
            {t.recentTierMod}
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
              {t.appsTitle}
            </h2>
            <span className="text-xs px-2 py-0.5 border border-[#5accf5]/40 text-[#5accf5] bg-[#140e10]">
              {t.appsCountSummary(config.apps.length, activeCount)}
            </span>
          </div>
          <p className="text-xs text-[#8a7f81] mt-1">
            {t.appsSubtitle}
          </p>
        </div>

        {/* Top Action Buttons with CodePen Glitch styling */}
        <div className="flex items-center gap-2.5 flex-wrap">
          <GlitchButton
            label={isScanning ? t.appsScanningBtn : t.appsScanBtn}
            variant="primary"
            size="sm"
            disabled={isScanning}
            icon={<ScanSearch className={`w-3.5 h-3.5 ${isScanning ? 'animate-spin' : ''}`} />}
            onClick={handleStartScan}
          />

          <GlitchButton
            label={t.appsAddManualBtn}
            variant="outline"
            size="sm"
            icon={<Plus className="w-3.5 h-3.5 text-[#5accf5]" />}
            onClick={() => setShowAddModal(true)}
          />

          <GlitchButton
            label={t.appsCatalogBtn}
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
            placeholder={t.appsSearchPlaceholder}
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
            title="Grid"
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
            title="List"
          >
            <ListIcon className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Launcher Filter Tabs */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
        {[
          { id: 'all', label: t.appsTabAll(config.apps.length) },
          { id: 'steam', label: t.appsTabSteam(steamCount) },
          { id: 'epic games', label: t.appsTabEpic },
          { id: 'windows', label: t.appsTabWindows },
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
              {search ? t.appsNoGamesSearchTitle : t.appsNoGamesTitle}
            </h3>
            <p className="text-xs text-[#8a7f81] max-w-md mx-auto">
              {t.appsNoGamesSubtitle}
            </p>
          </div>
          {!search && (
            <div className="flex items-center justify-center gap-3 pt-2">
              <GlitchButton
                label={t.appsScanDisksBtn}
                variant="primary"
                size="sm"
                onClick={handleStartScan}
              />
              <GlitchButton
                label={t.appsBrowseCatalogBtn}
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
                <div className="relative z-10 p-2 flex items-center justify-between flex-wrap gap-1">
                  {getHdrBadge(app.hdr_type)}
                  <div className="flex items-center gap-1">
                    {app.path && pathStatus[app.path] === false && (
                      <span
                        className="px-1 py-0.2 text-[8px] font-mono text-rose-300 bg-rose-950/90 border border-rose-500/60 uppercase flex items-center gap-0.5"
                        title={t.appsPathMissingTooltip}
                      >
                        <AlertCircle className="w-2.5 h-2.5" />
                        {t.appsPathMissing}
                      </span>
                    )}
                    {app.launcher && (
                      <span className="px-1 py-0.2 text-[8px] font-mono text-[#b5a9ac] bg-black/60 border border-white/10 uppercase">
                        {app.launcher}
                      </span>
                    )}
                  </div>
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
                      {app.enabled ? t.appsStatusTracked : t.appsStatusPaused}
                    </button>

                    {/* Delete button */}
                    <button
                      onClick={() => handleDeleteApp(app.exe_name)}
                      className="p-1 text-[#8a7f81] hover:text-[#f55a6b] cursor-pointer transition-colors"
                      title={t.appsRemoveFromLibrary}
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
                      {app.path && pathStatus[app.path] === false && (
                        <span
                          className="text-[9px] px-1.5 py-0.2 bg-rose-950/70 border border-rose-500/50 text-rose-300 font-mono flex items-center gap-1"
                          title={t.appsPathMissingTooltip}
                        >
                          <AlertCircle className="w-3 h-3" />
                          {t.appsPathMissing}
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
                    {app.enabled ? t.appsStatusTracked : t.appsStatusPaused}
                  </button>

                  <button
                    onClick={() => handleDeleteApp(app.exe_name)}
                    className="p-1.5 text-[#8a7f81] hover:text-[#f55a6b] cursor-pointer transition-colors"
                    title={t.appsRemoveFromLibrary}
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
      {showScanModal && (() => {
        const hdrGames = scannedGames.filter((g) => g.enabled);
        const sdrGames = scannedGames.filter((g) => !g.enabled);
        const selectedCount = Object.values(selectedToImport).filter(Boolean).length;

        return (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
            <div className="bg-[#0f0b0b] border-2 border-[#f55a6b] max-w-2xl w-full p-6 space-y-4 relative shadow-[0_0_30px_rgba(245,90,107,0.4)] max-h-[90vh] flex flex-col">
              <div className="flex items-center justify-between border-b border-[#f55a6b]/30 pb-3">
                <div className="flex items-center gap-2">
                  <ScanSearch className="w-5 h-5 text-[#5accf5]" />
                  <h3 className="glitch-title-bar px-2 py-0.5 text-xs font-bold uppercase">
                    {t.scanModalTitle(scannedGames.length)}
                  </h3>
                </div>
                <button
                  onClick={() => setShowScanModal(false)}
                  className="text-[#8a7f81] hover:text-white cursor-pointer"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-xs">
                <p className="text-[#8a7f81]">
                  {t.scanModalSubtitle}
                </p>
                <div className="flex items-center gap-2 flex-shrink-0">
                  <button
                    type="button"
                    onClick={selectAllHdr}
                    className="text-[10px] uppercase px-2 py-1 bg-[#120d0e] border border-[#5accf5]/50 text-[#5accf5] hover:bg-[#5accf5]/10 cursor-pointer font-mono"
                  >
                    {t.scanModalSelectAllHdr}
                  </button>
                  <button
                    type="button"
                    onClick={selectAll}
                    className="text-[10px] uppercase px-2 py-1 bg-[#120d0e] border border-white/20 text-white hover:bg-white/10 cursor-pointer font-mono"
                  >
                    {t.scanModalSelectAll}
                  </button>
                  <button
                    type="button"
                    onClick={deselectAll}
                    className="text-[10px] uppercase px-2 py-1 bg-[#120d0e] border border-white/10 text-[#8a7f81] hover:text-white cursor-pointer font-mono"
                  >
                    {t.scanModalDeselectAll}
                  </button>
                </div>
              </div>

              <div className="flex-1 overflow-y-auto space-y-4 pr-1">
                {/* SECTION 1: HDR Games (Top, Pre-selected) */}
                {hdrGames.length > 0 && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between px-2 py-1.5 bg-[#121c1f] border-l-2 border-[#5accf5] text-[#5accf5]">
                      <div className="flex items-center gap-2 text-xs font-bold uppercase">
                        <Sparkles className="w-4 h-4 text-[#5accf5]" />
                        <span>{t.scanModalSectionHdr(hdrGames.length)}</span>
                      </div>
                      <span className="text-[10px] font-mono opacity-80">[AUTO ON]</span>
                    </div>

                    <div className="space-y-1.5">
                      {hdrGames.map((game) => {
                        const isSelected = !!selectedToImport[game.exe_name];
                        const existing = findExistingApp(game);
                        const pathChanged = existing ? isPathDifferent(existing, game) : false;
                        const isAlreadyInLib = !!existing && !pathChanged;

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
                                ? 'bg-[#121c1f] border-[#5accf5] text-white shadow-[0_0_10px_rgba(90,204,245,0.15)]'
                                : 'bg-black/40 border-white/10 text-[#8a7f81] hover:border-white/30'
                            }`}
                          >
                            <div className="space-y-0.5 min-w-0 flex-1 pr-2">
                              <div className="font-bold flex items-center gap-2 flex-wrap">
                                <span>{game.name}</span>
                                {game.launcher && (
                                  <span className="text-[9px] px-1 bg-black border border-[#5accf5]/30 text-[#5accf5]">
                                    {game.launcher}
                                  </span>
                                )}
                                <span className="text-[9px] px-1.5 py-0.2 bg-[#5accf5]/10 border border-[#5accf5]/50 text-[#5accf5] font-mono uppercase">
                                  {game.hdr_type === 'autohdr' ? 'Auto HDR' : 'Native HDR'}
                                </span>

                                {/* Status Badges */}
                                {pathChanged && (
                                  <span className="text-[9px] px-1.5 py-0.2 bg-amber-500/15 border border-amber-500/60 text-amber-300 font-mono font-bold uppercase tracking-wider flex items-center gap-1">
                                    <RefreshCw className="w-2.5 h-2.5" />
                                    {t.scanModalStatusPathUpdate}
                                  </span>
                                )}
                                {isAlreadyInLib && (
                                  <span className="text-[9px] px-1.5 py-0.2 bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 font-mono uppercase">
                                    ✓ {t.scanModalStatusInLibrary}
                                  </span>
                                )}
                                {!existing && (
                                  <span className="text-[9px] px-1.5 py-0.2 bg-[#5accf5]/15 border border-[#5accf5]/70 text-[#5accf5] font-mono font-bold uppercase tracking-wider">
                                    ★ {t.scanModalStatusNew}
                                  </span>
                                )}
                              </div>
                              <div className="text-[10px] text-[#5accf5] font-mono truncate">[{game.exe_name}]</div>
                              {pathChanged && game.path && (
                                <div className="text-[9px] text-amber-300/80 font-mono truncate" title={game.path}>
                                  ➔ {t.scanModalNewLocation}: {game.path}
                                </div>
                              )}
                            </div>

                            <div
                              className={`w-4 h-4 border flex items-center justify-center shrink-0 transition-colors ${
                                isSelected
                                  ? 'bg-[#5accf5] border-[#5accf5] text-black'
                                  : 'border-[#8a7f81]'
                              }`}
                            >
                              {isSelected && <Check className="w-3 h-3 stroke-[3]" />}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}

                {/* SECTION 2: SDR Games (Bottom, Unchecked by default) */}
                {sdrGames.length > 0 && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between px-2 py-1.5 bg-[#120d0e] border-l-2 border-[#8a7f81] text-[#8a7f81]">
                      <div className="flex items-center gap-2 text-xs font-bold uppercase">
                        <Gamepad2 className="w-4 h-4 text-[#8a7f81]" />
                        <span>{t.scanModalSectionSdr(sdrGames.length)}</span>
                      </div>
                      <span className="text-[10px] font-mono opacity-80">[AUTO OFF]</span>
                    </div>

                    <div className="space-y-1.5">
                      {sdrGames.map((game) => {
                        const isSelected = !!selectedToImport[game.exe_name];
                        const existing = findExistingApp(game);
                        const pathChanged = existing ? isPathDifferent(existing, game) : false;
                        const isAlreadyInLib = !!existing && !pathChanged;

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
                            <div className="space-y-0.5 min-w-0 flex-1 pr-2">
                              <div className="font-bold flex items-center gap-2 flex-wrap">
                                <span>{game.name}</span>
                                {game.launcher && (
                                  <span className="text-[9px] px-1 bg-black border border-white/20 text-[#8a7f81]">
                                    {game.launcher}
                                  </span>
                                )}
                                <span className="text-[9px] px-1.5 py-0.2 bg-white/5 border border-white/10 text-[#8a7f81] font-mono uppercase">
                                  SDR
                                </span>

                                {/* Status Badges for SDR */}
                                {pathChanged && (
                                  <span className="text-[9px] px-1.5 py-0.2 bg-amber-500/15 border border-amber-500/60 text-amber-300 font-mono font-bold uppercase tracking-wider flex items-center gap-1">
                                    <RefreshCw className="w-2.5 h-2.5" />
                                    {t.scanModalStatusPathUpdate}
                                  </span>
                                )}
                                {isAlreadyInLib && (
                                  <span className="text-[9px] px-1.5 py-0.2 bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 font-mono uppercase">
                                    ✓ {t.scanModalStatusInLibrary}
                                  </span>
                                )}
                              </div>
                              <div className="text-[10px] text-[#8a7f81] font-mono truncate">[{game.exe_name}]</div>
                              {pathChanged && game.path && (
                                <div className="text-[9px] text-amber-300/80 font-mono truncate" title={game.path}>
                                  ➔ {t.scanModalNewLocation}: {game.path}
                                </div>
                              )}
                            </div>

                            <div
                              className={`w-4 h-4 border flex items-center justify-center shrink-0 transition-colors ${
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
                  </div>
                )}
              </div>

              <div className="flex items-center justify-between border-t border-[#f55a6b]/30 pt-3">
                <div className="text-xs text-[#8a7f81]">
                  {selectedCount} vybráno / selected
                </div>
                <div className="flex items-center gap-3">
                  <GlitchButton
                    label={t.scanModalCancel}
                    variant="outline"
                    size="sm"
                    onClick={() => setShowScanModal(false)}
                  />
                  <GlitchButton
                    label={`${t.scanModalAddSelected} (${selectedCount})`}
                    variant="primary"
                    size="sm"
                    onClick={handleConfirmImport}
                  />
                </div>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Manual Add Modal & File Picker */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
          <div className="bg-[#0f0b0b] border-2 border-[#f55a6b] max-w-lg w-full p-6 space-y-4 relative shadow-[0_0_30px_rgba(245,90,107,0.4)]">
            <div className="flex items-center justify-between border-b border-[#f55a6b]/30 pb-3">
              <h3 className="glitch-title-bar px-2 py-0.5 text-xs font-bold uppercase">
                {t.manualModalTitle}
              </h3>
              <button
                onClick={() => {
                  setShowAddModal(false);
                  setNewPath('');
                  setIsHdrMatched(false);
                }}
                className="text-[#8a7f81] hover:text-white cursor-pointer"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Interactive File Dropzone & Browse Button */}
            <div
              onClick={handleBrowseExe}
              onDragOver={(e) => {
                e.preventDefault();
                setIsDraggingOver(true);
              }}
              onDragLeave={() => setIsDraggingOver(false)}
              onDrop={(e) => {
                e.preventDefault();
                setIsDraggingOver(false);
              }}
              className={`p-4 border-2 border-dashed cursor-pointer text-center transition-all ${
                isDraggingOver
                  ? 'border-[#5accf5] bg-[#5accf5]/15 text-white shadow-[0_0_15px_rgba(90,204,245,0.3)]'
                  : 'border-[#f55a6b]/40 hover:border-[#f55a6b] bg-[#120d0e]/60 text-[#8a7f81] hover:text-white'
              }`}
            >
              <UploadCloud className={`w-7 h-7 mx-auto mb-1.5 transition-colors ${isDraggingOver ? 'text-[#5accf5]' : 'text-[#f55a6b]'}`} />
              <div className="text-xs font-bold uppercase text-white flex items-center justify-center gap-1.5">
                <FolderOpen className="w-3.5 h-3.5 text-[#5accf5]" />
                <span>{t.manualModalBrowseBtn}</span>
              </div>
              <p className="text-[10px] mt-1 text-[#8a7f81]">
                {t.manualModalDragDropHint}
              </p>
            </div>

            {/* Catalog Match Banner */}
            {isHdrMatched && (
              <div className="p-2 bg-[#121c1f] border border-[#5accf5] text-[#5accf5] flex items-center gap-2 text-xs">
                <Sparkles className="w-4 h-4 flex-shrink-0" />
                <span className="font-bold uppercase tracking-wider">{t.manualModalDetectedBadge}</span>
              </div>
            )}

            <form onSubmit={handleAddCustomApp} className="space-y-4">
              <div className="space-y-1">
                <label className="text-xs uppercase text-[#8a7f81]">{t.manualModalName}</label>
                <input
                  type="text"
                  required
                  placeholder="e.g. Silent Hill 2 / Cyberpunk 2077"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none"
                />
              </div>

              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <label className="text-xs uppercase text-[#8a7f81]">{t.manualModalExe}</label>
                  <button
                    type="button"
                    onClick={handleBrowseExe}
                    className="text-[10px] text-[#5accf5] hover:underline flex items-center gap-1 cursor-pointer font-mono"
                  >
                    <FolderOpen className="w-3 h-3" />
                    {t.manualModalBrowseBtn}
                  </button>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    required
                    placeholder="e.g. SHProto-Win64-Shipping.exe"
                    value={newExe}
                    onChange={(e) => setNewExe(e.target.value)}
                    className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none font-mono"
                  />
                  <button
                    type="button"
                    onClick={handleBrowseExe}
                    className="px-3 py-1.5 bg-[#1c0f12] border border-[#f55a6b] text-[#f55a6b] hover:bg-[#f55a6b] hover:text-black text-xs font-bold flex items-center gap-1 cursor-pointer transition-colors"
                  >
                    <FolderOpen className="w-4 h-4" />
                  </button>
                </div>
                {newPath && (
                  <div className="text-[9px] text-[#5accf5] font-mono truncate pt-0.5" title={newPath}>
                    {t.manualModalPath}: {newPath}
                  </div>
                )}
              </div>

              <div className="space-y-1">
                <label className="text-xs uppercase text-[#8a7f81]">{t.manualModalType}</label>
                <select
                  value={newType}
                  onChange={(e) => setNewType(e.target.value as HdrType)}
                  className="w-full px-3 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white focus:outline-none"
                >
                  <option value="native">{t.catalogTierNative}</option>
                  <option value="autohdr">{t.catalogTierAutoHdr}</option>
                  <option value="custom">{t.catalogTierCustom}</option>
                  <option value="media">{t.catalogTierMedia}</option>
                </select>
              </div>

              <div className="flex items-center justify-end gap-3 pt-3">
                <GlitchButton
                  type="button"
                  label={t.manualModalCancel}
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setShowAddModal(false);
                    setNewPath('');
                    setIsHdrMatched(false);
                  }}
                />
                <GlitchButton
                  type="submit"
                  label={t.manualModalSubmit}
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
