import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { MonitorInfo, AppConfig, HdrStatePayload } from './types';
import { Dashboard } from './components/Dashboard';
import { AppsManager } from './components/AppsManager';
import { CatalogBrowser } from './components/CatalogBrowser';
import { RunningProcesses } from './components/RunningProcesses';
import { Settings } from './components/Settings';
import {
  Flame,
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
      setStatus(event.payload);
      refreshMonitors();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <div
      className={`min-h-screen transition-colors duration-300 flex flex-col font-sans ${
        isDark
          ? 'bg-slate-950 text-slate-100'
          : 'bg-slate-100 text-slate-900'
      }`}
      style={{
        background: isDark
          ? 'radial-gradient(circle at 10% 20%, rgba(20, 30, 60, 0.6) 0%, rgba(9, 13, 22, 0.95) 90%)'
          : 'radial-gradient(circle at 10% 20%, rgba(220, 235, 255, 0.7) 0%, rgba(241, 245, 249, 0.95) 90%)',
      }}
    >
      {/* Top Header Bar */}
      <header
        className={`sticky top-0 z-30 px-6 py-3.5 border-b backdrop-blur-xl transition-colors glass-panel ${
          isDark
            ? 'bg-slate-950/70 border-white/10'
            : 'bg-white/80 border-slate-200'
        }`}
      >
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div
              className={`p-2 rounded-xl transition-all duration-300 ${
                status.is_hdr_active
                  ? 'bg-gradient-to-tr from-amber-500 via-rose-500 to-purple-600 text-white shadow-lg shadow-rose-500/25'
                  : isDark
                  ? 'bg-slate-800 text-cyan-400'
                  : 'bg-slate-200 text-cyan-600'
              }`}
            >
              <Flame className="w-5 h-5" />
            </div>

            <div>
              <div className="flex items-center gap-2">
                <h1 className="font-extrabold text-base tracking-tight">
                  HDR Auto-Switch
                </h1>
                <span
                  className={`text-[10px] font-bold px-1.5 py-0.5 rounded tracking-wide ${
                    status.is_hdr_active
                      ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                      : isDark
                      ? 'bg-slate-800 text-slate-400'
                      : 'bg-slate-200 text-slate-600'
                  }`}
                >
                  {status.is_hdr_active ? 'HDR ON' : 'SDR'}
                </span>
              </div>
              <p className="text-[11px] text-slate-400">
                Automatické přepínání HDR pro Windows
              </p>
            </div>
          </div>

          {/* Navigation Tabs */}
          <nav className="flex items-center gap-1">
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
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all ${
                    isActive
                      ? isDark
                        ? 'bg-white/10 text-white shadow-sm border border-white/10'
                        : 'bg-slate-200 text-slate-900 shadow-sm border border-slate-300'
                      : isDark
                      ? 'text-slate-400 hover:text-white hover:bg-white/5'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-200/60'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Theme Switch */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setIsDark(!isDark)}
              className={`p-2 rounded-xl border transition-all cursor-pointer ${
                isDark
                  ? 'border-white/10 bg-slate-900 text-amber-400 hover:bg-slate-800'
                  : 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'
              }`}
              title={isDark ? 'Světlý režim' : 'Tmavý režim'}
            >
              {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
          </div>
        </div>
      </header>

      {/* Main Content Body */}
      <main className="flex-1 max-w-6xl w-full mx-auto p-6">
        {activeTab === 'dashboard' && (
          <Dashboard
            status={status}
            monitors={monitors}
            config={config}
            onRefreshMonitors={refreshMonitors}
            onManualToggle={(enable) =>
              setStatus((prev) => ({ ...prev, is_hdr_active: enable }))
            }
            isDark={isDark}
          />
        )}

        {activeTab === 'apps' && (
          <AppsManager
            config={config}
            onUpdateConfig={setConfig}
            onNavigateToCatalog={() => setActiveTab('catalog')}
            isDark={isDark}
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

      {/* Status Bar Footer */}
      <footer
        className={`px-6 py-2.5 border-t text-xs transition-colors backdrop-blur-md glass-panel ${
          isDark
            ? 'bg-slate-950/60 border-white/5 text-slate-500'
            : 'bg-white/60 border-slate-200 text-slate-500'
        }`}
      >
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                status.is_hdr_active ? 'bg-rose-500 animate-pulse' : 'bg-slate-500'
              }`}
            />
            <span>
              {status.is_hdr_active
                ? `HDR aktivní ${
                    status.current_app_name ? `(${status.current_app_name})` : ''
                  }`
                : 'SDR režim (Žádná HDR hra)'}
            </span>
          </div>

          <div className="flex items-center gap-4 text-[11px]">
            <span>Sledováno {config.apps.filter((a) => a.enabled).length} her</span>
            <span>•</span>
            <span>Minimalizováno v liště (System Tray)</span>
            <span>•</span>
            <span>0.0% CPU</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
