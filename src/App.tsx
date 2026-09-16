import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import gsap from 'gsap';
import {
  MonitorInfo,
  AppConfig,
  HdrStatePayload,
  ActivityLogEntry,
  RecentGameSession,
} from './types';
import { Dashboard } from './components/Dashboard';
import { AppsManager } from './components/AppsManager';
import { CatalogBrowser } from './components/CatalogBrowser';
import { RunningProcesses } from './components/RunningProcesses';
import { Settings } from './components/Settings';
import { HdrLogo } from './components/HdrLogo';
import { GlitchNavItem } from './components/GlitchNavItem';
import { Sun, Moon, Globe } from 'lucide-react';
import { I18nContext, Language, dictionaries, detectDefaultLanguage } from './i18n';
import './App.css';

type Tab = 'dashboard' | 'apps' | 'catalog' | 'processes' | 'settings';

const DEFAULT_RECENT_GAMES: RecentGameSession[] = [
  {
    exe: 'bodycam.exe',
    name: 'Bodycam',
    steam_id: '2406770',
    launcher: 'Steam',
    hdr_type: 'native',
    hdr_tier_label: 'Nativní HDR10',
    last_switched_at: '14:27',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
  {
    exe: 'acs.exe',
    name: 'Assetto Corsa',
    steam_id: '244210',
    launcher: 'Steam',
    hdr_type: 'mod',
    hdr_tier_label: 'HDR Mod / Pure',
    last_switched_at: '13:45',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
  {
    exe: 'bf2042.exe',
    name: 'Battlefield 6',
    steam_id: '1517290',
    launcher: 'Steam',
    hdr_type: 'native',
    hdr_tier_label: 'Nativní HDR10',
    last_switched_at: '12:10',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
  {
    exe: 'beamng.drive.x64.exe',
    name: 'BeamNG.drive',
    steam_id: '284160',
    launcher: 'Steam',
    hdr_type: 'autohdr',
    hdr_tier_label: 'Windows Auto HDR',
    last_switched_at: '11:05',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
  {
    exe: 'enshrouded.exe',
    name: 'Enshrouded',
    steam_id: '1203620',
    launcher: 'Steam',
    hdr_type: 'native',
    hdr_tier_label: 'Nativní HDR10',
    last_switched_at: 'Včera',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
  {
    exe: 'forzahorizon5.exe',
    name: 'Forza Horizon 6',
    steam_id: '1551360',
    launcher: 'Steam',
    hdr_type: 'native',
    hdr_tier_label: 'Nativní HDR10',
    last_switched_at: 'Včera',
    hook_status: 'switched_off',
    hook_message: 'WinEventHook: HDR zapnuto -> SDR obnoveno',
  },
];

const IGNORED_SYSTEM_EXES = new Set([
  'snippingtool.exe',
  'screenclippinghost.exe',
  'explorer.exe',
  'chrome.exe',
  'msedge.exe',
  'firefox.exe',
  'applicationframehost.exe',
  'gamingservicesui.exe',
  'searchhost.exe',
  'shellexperiencehost.exe',
  'startmenuexperiencehost.exe',
  'taskmgr.exe',
  'systemsettings.exe',
  'hdr-autoswitch.exe',
  'tauri-app.exe',
]);

const normalizeKey = (str?: string | null) => {
  if (!str) return '';
  return str
    .toLowerCase()
    .replace('.exe', '')
    .replace(/-win64-shipping|_win64_shipping|-shipping|_shipping|_dx12|_dx11|_vk/g, '')
    .replace(/[^a-z0-9]/g, '');
};

const isSameGame = (
  a: { exe?: string | null; name?: string | null; steam_id?: string | null },
  b: { exe?: string | null; name?: string | null; steam_id?: string | null }
): boolean => {
  if (a.steam_id && b.steam_id && a.steam_id === b.steam_id) return true;
  if (a.exe && b.exe && a.exe.toLowerCase() === b.exe.toLowerCase()) return true;

  const normNameA = normalizeKey(a.name);
  const normNameB = normalizeKey(b.name);
  if (normNameA.length > 2 && normNameB.length > 2 && normNameA === normNameB) return true;

  const normExeA = normalizeKey(a.exe);
  const normExeB = normalizeKey(b.exe);
  if (normExeA.length > 2 && normExeB.length > 2 && normExeA === normExeB) return true;

  if (normExeA.length > 2 && normNameB.length > 2 && normExeA === normNameB) return true;
  if (normNameA.length > 2 && normExeB.length > 2 && normNameA === normExeB) return true;

  return false;
};

function dedupeRecentGameList(list: RecentGameSession[]): RecentGameSession[] {
  const result: RecentGameSession[] = [];
  for (const item of list) {
    if (!item.exe || IGNORED_SYSTEM_EXES.has(item.exe.toLowerCase())) continue;
    const existingIndex = result.findIndex((r) => isSameGame(r, item));
    if (existingIndex >= 0) {
      const existing = result[existingIndex];
      result[existingIndex] = {
        ...existing,
        name: existing.name || item.name,
        steam_id: existing.steam_id || item.steam_id,
        launcher: existing.launcher || item.launcher,
        hdr_tier_label: existing.hdr_tier_label || item.hdr_tier_label,
        hdr_type: existing.hdr_type || item.hdr_type,
      };
    } else {
      result.push(item);
    }
  }
  return result;
}

export default function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [isDark, setIsDark] = useState(true);
  const [lang, setLang] = useState<Language>(detectDefaultLanguage);

  const handleSetLang = (newLang: Language) => {
    setLang(newLang);
    localStorage.setItem('hdr_lang', newLang);
  };

  const t = dictionaries[lang];

  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [config, setConfig] = useState<AppConfig>({
    target_monitor: 'all',
    alt_tab_delay_seconds: 2,
    exit_only_hdr: true,
    notifications_enabled: true,
    autostart: false,
    start_minimized: false,
    auto_detect_new_games: true,
    auto_sync_database: true,
    switch_method: 'native',
    blacklist: [],
    apps: [],
  });

  const [status, setStatus] = useState<HdrStatePayload>({
    is_hdr_active: false,
    current_app_name: null,
    current_exe: null,
    switched_by_app: false,
    steam_id: null,
    launcher: null,
    hdr_type: null,
  });

  const [recentGames, setRecentGames] = useState<RecentGameSession[]>(() => {
    try {
      const saved = localStorage.getItem('hdr_recent_games');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed)) {
          const sanitized = dedupeRecentGameList(parsed);
          if (sanitized.length > 0) {
            localStorage.setItem('hdr_recent_games', JSON.stringify(sanitized));
            return sanitized;
          }
        }
      }
    } catch (e) {
      console.error('Error loading recent games:', e);
    }
    return DEFAULT_RECENT_GAMES;
  });

  const [activityLogs, setActivityLogs] = useState<ActivityLogEntry[]>([
    {
      id: '1',
      timestamp: new Date().toLocaleTimeString(),
      message: 'WinEventHook služba inicializována. Zero CPU režim aktivní.',
      type: 'system',
    },
    {
      id: '2',
      timestamp: new Date().toLocaleTimeString(),
      message: 'Sledování popředí oken běží — bleskový přechod HDR10 připraven.',
      type: 'info',
    },
  ]);

  const addLog = (message: string, type: ActivityLogEntry['type']) => {
    const entry: ActivityLogEntry = {
      id: Math.random().toString(36).substring(2, 9),
      timestamp: new Date().toLocaleTimeString(),
      message,
      type,
    };
    setActivityLogs((prev) => [entry, ...prev.slice(0, 19)]);
  };

  const refreshMonitors = async () => {
    try {
      const list: MonitorInfo[] = await invoke('get_monitors');
      setMonitors(list);
    } catch (err) {
      console.error('Failed to get monitors:', err);
    }
  };

  const refreshConfig = async () => {
    try {
      const conf: AppConfig = await invoke('get_config');
      setConfig(conf);
    } catch (err) {
      console.error('Failed to get config:', err);
    }
  };

  const refreshStatus = async () => {
    try {
      const stat: HdrStatePayload = await invoke('get_current_status');
      setStatus((prev) => ({
        ...prev,
        is_hdr_active: stat.is_hdr_active,
      }));
    } catch (err) {
      console.error('Failed to get current status:', err);
    }
  };

  useEffect(() => {
    refreshMonitors();
    refreshConfig();
    refreshStatus();

    // Listen for live HDR status changes from Rust WinEventHook
    const unlistenPromise = listen<HdrStatePayload>('hdr-status-changed', (event) => {
      const newStatus = event.payload;
      setStatus(newStatus);
      refreshMonitors();

      const currentTime = new Date().toLocaleTimeString('cs-CZ', {
        hour: '2-digit',
        minute: '2-digit',
      });

      if (newStatus.is_hdr_active) {
        addLog(
          newStatus.current_app_name
            ? `WinEventHook zachytil okno: ${newStatus.current_app_name} -> HDR aktivováno`
            : 'HDR aktivováno ručně.',
          'hdr_on'
        );

        // Update Recent Games telemetry (only for actual games detected by hook)
        if (
          newStatus.switched_by_app &&
          newStatus.current_exe &&
          !IGNORED_SYSTEM_EXES.has(newStatus.current_exe.toLowerCase())
        ) {
          setRecentGames((prev) => {
            const candidate = {
              exe: newStatus.current_exe,
              name: newStatus.current_app_name,
              steam_id: newStatus.steam_id,
            };

            const existing = prev.find((g) => isSameGame(g, candidate));

            const resolvedSteamId =
              newStatus.steam_id ||
              existing?.steam_id ||
              undefined;

            const resolvedLauncher =
              newStatus.launcher ||
              existing?.launcher ||
              (resolvedSteamId ? 'Steam' : undefined);

            const resolvedHdrType =
              newStatus.hdr_type ||
              existing?.hdr_type ||
              'native';

            const resolvedTierLabel =
              existing?.hdr_tier_label ||
              (resolvedHdrType === 'autohdr'
                ? 'Windows Auto HDR'
                : resolvedHdrType === 'mod'
                ? 'HDR Mod / Fix'
                : 'Nativní HDR10');

            const updatedSession: RecentGameSession = {
              exe: newStatus.current_exe!,
              name: newStatus.current_app_name || existing?.name || newStatus.current_exe!,
              steam_id: resolvedSteamId,
              launcher: resolvedLauncher,
              hdr_type: resolvedHdrType,
              hdr_tier_label: resolvedTierLabel,
              last_switched_at: currentTime,
              hook_status: 'active',
              hook_message: 'WinEventHook zachytil okno -> HDR zapnuto',
            };

            const filtered = prev.filter(
              (g) =>
                !isSameGame(g, updatedSession) &&
                !IGNORED_SYSTEM_EXES.has(g.exe.toLowerCase())
            );

            const newList = [updatedSession, ...filtered].slice(0, 10);
            try {
              localStorage.setItem('hdr_recent_games', JSON.stringify(newList));
            } catch (e) {
              console.error(e);
            }
            return newList;
          });
        }
      } else {
        addLog('WinEventHook: Návrat do SDR (okno opuštěno).', 'hdr_off');

        // Mark active game as switched_off
        setRecentGames((prev) => {
          const newList = prev.map((g, idx) =>
            idx === 0 && g.hook_status === 'active'
              ? {
                  ...g,
                  hook_status: 'switched_off' as const,
                  hook_message: `Hook zafungoval: Návrat do SDR (${currentTime})`,
                }
              : g
          );
          try {
            localStorage.setItem('hdr_recent_games', JSON.stringify(newList));
          } catch (e) {
            console.error(e);
          }
          return newList;
        });
      }
    });

    // Listen for apps updates (e.g. background auto-scan or hook auto-enrollment)
    const unlistenAppsPromise = listen('apps-updated', () => {
      refreshConfig();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
      unlistenAppsPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Zero-overhead GUI Hibernation when minimized or hidden to system tray:
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.hidden) {
        // App is hidden in system tray or minimized: pause GSAP ticker & animation loops for 0.0% CPU overhead
        gsap.ticker.sleep();
      } else {
        // Window restored / focused from tray: wake up GSAP ticker and refresh state
        gsap.ticker.wake();
        refreshMonitors();
        refreshStatus();
        refreshConfig();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  return (
    <I18nContext.Provider value={{ lang, setLang: handleSetLang, t }}>
      <div
        className={`min-h-screen flex flex-col transition-colors duration-200 font-mono ${
          isDark ? 'bg-retro-dark text-[#e5e0e1]' : 'bg-retro-light text-slate-900'
        }`}
      >
        {/* Top Header Bar - CodePen Retro Glitch Aesthetic */}
        <header
          className={`sticky top-0 z-30 px-6 py-2.5 border-b transition-colors ${
            isDark
              ? 'bg-[#0f0b0b]/95 border-[#f55a6b]/30'
              : 'bg-white/95 border-[#f55a6b]/30 shadow-xs'
          }`}
        >
          <div className="max-w-7xl mx-auto flex items-center justify-between gap-4">
            {/* Brand Logo & Name with Solid Glitch Title Bar */}
            <div className="flex items-center gap-3">
              <div className="p-1 border border-[#f55a6b]/40 bg-[#180e10]">
                <HdrLogo size={28} active={status.is_hdr_active} />
              </div>

              <div className="flex items-center gap-2.5">
                <h1 className="glitch-title-bar px-2 py-0.5 text-xs font-bold tracking-wider inline-block">
                  {t.appTitle}
                </h1>

                {/* Status Pill Badge */}
                <div
                  className={`inline-flex items-center gap-1.5 px-2 py-0.5 text-[11px] font-bold tracking-wider border uppercase transition-all ${
                    status.is_hdr_active
                      ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                      : 'bg-[#180e10] text-[#5accf5] border-[#5accf5]/50'
                  }`}
                >
                  <span
                    className={`w-1.5 h-1.5 ${
                      status.is_hdr_active
                        ? 'bg-[#0f0b0b] animate-status-pulse'
                        : 'bg-[#5accf5]'
                    }`}
                  />
                  <span>{status.is_hdr_active ? t.hdrActive : t.sdrStandby}</span>
                </div>
              </div>
            </div>

            {/* Glitch Navigation Bar (GSAP SVG Displacement from CodePen) */}
            <nav className="flex items-center gap-2">
              <GlitchNavItem
                label={t.navOverview}
                isActive={activeTab === 'dashboard'}
                onClick={() => setActiveTab('dashboard')}
                width={lang === 'en' ? 125 : 125}
                height={36}
              />
              <GlitchNavItem
                label={t.navApps}
                count={config.apps.length}
                isActive={activeTab === 'apps'}
                onClick={() => setActiveTab('apps')}
                width={lang === 'en' ? 140 : 145}
                height={36}
              />
              <GlitchNavItem
                label={t.navCatalog}
                isActive={activeTab === 'catalog'}
                onClick={() => setActiveTab('catalog')}
                width={lang === 'en' ? 140 : 140}
                height={36}
              />
              <GlitchNavItem
                label={t.navProcesses}
                isActive={activeTab === 'processes'}
                onClick={() => setActiveTab('processes')}
                width={lang === 'en' ? 150 : 135}
                height={36}
              />
              <GlitchNavItem
                label={t.navSettings}
                isActive={activeTab === 'settings'}
                onClick={() => setActiveTab('settings')}
                width={lang === 'en' ? 125 : 125}
                height={36}
              />
            </nav>

            {/* Right Controls: Language & Theme Switch */}
            <div className="flex items-center gap-2">
              {/* Language Switch Button */}
              <button
                onClick={() => handleSetLang(lang === 'cs' ? 'en' : 'cs')}
                className={`px-2 py-1 border text-xs font-bold transition-all cursor-pointer flex items-center gap-1.5 ${
                  isDark
                    ? 'border-[#f55a6b]/30 bg-[#180e10] text-[#5accf5] hover:border-[#f55a6b] hover:shadow-[0_0_10px_rgba(245,90,107,0.3)]'
                    : 'border-[#f55a6b]/40 bg-white text-[#f55a6b] hover:bg-slate-50'
                }`}
                title={t.langToggle}
              >
                <Globe className="w-3.5 h-3.5" />
                <span>{lang.toUpperCase()}</span>
              </button>

              <button
                onClick={() => setIsDark(!isDark)}
                className={`p-1.5 border transition-all cursor-pointer ${
                  isDark
                    ? 'border-[#f55a6b]/30 bg-[#180e10] text-[#5accf5] hover:border-[#f55a6b] hover:shadow-[0_0_10px_rgba(245,90,107,0.4)]'
                    : 'border-[#f55a6b]/40 bg-white text-[#f55a6b] hover:bg-slate-50'
                }`}
                title={t.themeToggle}
              >
                {isDark ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
              </button>
            </div>
          </div>
        </header>

      {/* Main Content Body */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-6">
        {activeTab === 'dashboard' && (
          <Dashboard
            status={status}
            monitors={monitors}
            config={config}
            activityLogs={activityLogs}
            recentGames={recentGames}
            onRefreshMonitors={refreshMonitors}
            onManualToggle={(enable) => {
              setStatus((prev) => ({ ...prev, is_hdr_active: enable }));
              addLog(
                enable
                  ? 'HDR zapnuto ručně přes ovládací panel.'
                  : 'HDR vypnuto ručně.',
                enable ? 'hdr_on' : 'hdr_off'
              );
            }}
            onNavigateToApps={() => setActiveTab('apps')}
            onUpdateConfig={setConfig}
            isDark={isDark}
          />
        )}

        {activeTab === 'apps' && (
          <AppsManager
            config={config}
            onUpdateConfig={setConfig}
            isDark={isDark}
            onNavigateToCatalog={() => setActiveTab('catalog')}
          />
        )}

        {activeTab === 'catalog' && (
          <CatalogBrowser
            config={config}
            onUpdateConfig={setConfig}
            isDark={isDark}
          />
        )}

        {activeTab === 'processes' && (
          <RunningProcesses
            config={config}
            onUpdateConfig={setConfig}
            isDark={isDark}
          />
        )}

        {activeTab === 'settings' && (
          <Settings
            config={config}
            monitors={monitors}
            onUpdateConfig={setConfig}
            isDark={isDark}
          />
        )}
      </main>
    </div>
  </I18nContext.Provider>
  );
}
