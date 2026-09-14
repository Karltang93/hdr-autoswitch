import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { MonitorInfo, AppConfig, HdrStatePayload, ActivityLogEntry } from './types';
import { Dashboard } from './components/Dashboard';
import { AppsManager } from './components/AppsManager';
import { CatalogBrowser } from './components/CatalogBrowser';
import { RunningProcesses } from './components/RunningProcesses';
import { Settings } from './components/Settings';
import { HdrLogo } from './components/HdrLogo';
import {
  Gamepad2,
  Sliders,
  AppWindow,
  Sun,
  Moon,
  Tv,
  Compass,
} from 'lucide-react';
import './App.css';

type Tab = 'dashboard' | 'apps' | 'catalog' | 'processes' | 'settings';

export default function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [isDark, setIsDark] = useState(true);

  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [config, setConfig] = useState<AppConfig>({
    target_monitor: 'all',
    alt_tab_delay_seconds: 2,
    notifications_enabled: true,
    autostart: false,
    switch_method: 'native',
    blacklist: [],
    apps: [],
  });

  const [status, setStatus] = useState<HdrStatePayload>({
    is_hdr_active: false,
    current_app_name: null,
    current_exe: null,
    switched_by_app: false,
  });

  const [activityLogs, setActivityLogs] = useState<ActivityLogEntry[]>([
    {
      id: '1',
      timestamp: new Date().toLocaleTimeString(),
      message: 'Služba WinEventHook byla úspěšně spuštěna na pozadí.',
      type: 'system',
    },
    {
      id: '2',
      timestamp: new Date().toLocaleTimeString(),
      message: 'Detekce her a systémových oken je aktivní.',
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

      if (newStatus.is_hdr_active) {
        addLog(
          newStatus.current_app_name
            ? `HDR aktivováno pro hru: ${newStatus.current_app_name}`
            : 'HDR aktivováno ručně.',
          'hdr_on'
        );
      } else {
        addLog('HDR vypnuto (návrat do SDR).', 'hdr_off');
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <div
      className={`min-h-screen flex flex-col transition-colors duration-200 ${
        isDark ? 'bg-gaming-dark text-slate-100' : 'bg-gaming-light text-slate-900'
      }`}
    >
      {/* Top Header Bar - Clean Modern Studio Gaming Aesthetic */}
      <header
        className={`sticky top-0 z-30 px-6 py-3.5 border-b glass-panel transition-colors ${
          isDark
            ? 'bg-[#0a0d15]/85 border-white/[0.08]'
            : 'bg-white/90 border-slate-200/80 shadow-xs'
        }`}
      >
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          {/* Brand Logo & Name */}
          <div className="flex items-center gap-3">
            <HdrLogo size={36} active={status.is_hdr_active} />

            <div className="flex items-center gap-3">
              <span className="font-extrabold text-base tracking-tight bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent">
                HDR Auto-Switch
              </span>

              {/* Status Pill Badge with Neon Glow */}
              <div
                className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide transition-all ${
                  status.is_hdr_active
                    ? 'bg-rose-500/20 text-rose-300 border border-rose-500/40 neon-glow-rose'
                    : isDark
                    ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/20'
                    : 'bg-slate-100 text-slate-700 border border-slate-200 shadow-xs'
                }`}
              >
                <span
                  className={`w-2 h-2 rounded-full ${
                    status.is_hdr_active
                      ? 'bg-rose-400 animate-status-pulse'
                      : 'bg-cyan-400'
                  }`}
                />
                <span>{status.is_hdr_active ? 'HDR Aktivní' : 'SDR Standby'}</span>
              </div>
            </div>
          </div>

          {/* Segmented Navigation Bar */}
          <nav
            className={`flex items-center gap-1 p-1 rounded-xl border ${
              isDark
                ? 'bg-white/[0.03] border-white/[0.06]'
                : 'bg-slate-100 border-slate-200'
            }`}
          >
            {[
              { id: 'dashboard', label: 'Přehled', icon: Tv },
              {
                id: 'apps',
                label: `Moje hry (${config.apps.length})`,
                icon: Gamepad2,
              },
              { id: 'catalog', label: 'Databáze her', icon: Compass },
              { id: 'processes', label: 'Běžící okna', icon: AppWindow },
              { id: 'settings', label: 'Nastavení', icon: Sliders },
            ].map((tab) => {
              const Icon = tab.icon;
              const isActive = activeTab === tab.id;

              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id as Tab)}
                  className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-all duration-150 ${
                    isActive
                      ? isDark
                        ? 'bg-white/10 text-white shadow-sm border border-white/15'
                        : 'bg-white text-slate-900 shadow-xs border border-slate-300'
                      : isDark
                      ? 'text-slate-400 hover:text-white hover:bg-white/[0.04] border border-transparent'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-200/60 border border-transparent'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Right Controls: Theme Switch */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setIsDark(!isDark)}
              className={`p-2 rounded-xl border transition-all cursor-pointer ${
                isDark
                  ? 'border-white/[0.08] bg-white/[0.03] text-amber-400 hover:bg-white/[0.08] hover:border-amber-400/40'
                  : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-100 shadow-xs'
              }`}
              title={isDark ? 'Přepnout na světlý režim' : 'Přepnout na tmavý režim'}
            >
              {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
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
            onRefreshMonitors={refreshMonitors}
            onManualToggle={(enable) => {
              setStatus((prev) => ({ ...prev, is_hdr_active: enable }));
              addLog(enable ? 'HDR zapnuto ručně přes ovládací panel.' : 'HDR vypnuto ručně.', enable ? 'hdr_on' : 'hdr_off');
            }}
            onNavigateToApps={() => setActiveTab('apps')}
            onUpdateConfig={setConfig}
            isDark={isDark}
          />
        )}

        {activeTab === 'apps' && (
          <AppsManager
            config={config}
            onUpdateConfig={(newConf) => {
              setConfig(newConf);
              addLog('Konfigurace sledovaných her byla aktualizována.', 'info');
            }}
            onNavigateToCatalog={() => setActiveTab('catalog')}
            isDark={isDark}
          />
        )}

        {activeTab === 'catalog' && (
          <CatalogBrowser
            config={config}
            onUpdateConfig={(newConf) => {
              setConfig(newConf);
              addLog('Změna v seznamu sledovaných her z katalogu.', 'info');
            }}
            isDark={isDark}
          />
        )}

        {activeTab === 'processes' && (
          <RunningProcesses
            config={config}
            onUpdateConfig={(newConf) => {
              setConfig(newConf);
              addLog('Aplikace přidána do sledování z běžících procesů.', 'game');
            }}
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

      {/* Status Bar Footer */}
      <footer
        className={`px-6 py-2.5 border-t text-xs font-mono transition-colors glass-panel ${
          isDark
            ? 'bg-[#0a0d15]/85 border-white/[0.06] text-slate-400'
            : 'bg-white/80 border-slate-200 text-slate-600'
        }`}
      >
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <span className="relative flex h-2 w-2">
              <span
                className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${
                  status.is_hdr_active ? 'bg-rose-400' : 'bg-cyan-400'
                }`}
              />
              <span
                className={`relative inline-flex rounded-full h-2 w-2 ${
                  status.is_hdr_active ? 'bg-rose-500' : 'bg-cyan-500'
                }`}
              />
            </span>
            <span className="font-medium text-xs">
              {status.is_hdr_active
                ? `HDR Aktivní • ${status.current_app_name || 'Ruční přepnutí'}`
                : 'SDR Standby • WinEventHook sleduje aktivní okna'}
            </span>
          </div>

          <div className="flex items-center gap-4 text-xs text-slate-500">
            <span>{config.apps.filter((a) => a.enabled).length} sledovaných her</span>
            <span>•</span>
            <span>Zero CPU EventHook</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
