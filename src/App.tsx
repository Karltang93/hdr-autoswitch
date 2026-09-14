import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { MonitorInfo, AppConfig, HdrStatePayload } from './types';
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
  Cpu,
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
      className={`min-h-screen flex flex-col transition-colors duration-200 ${
        isDark ? 'bg-studio-dark text-slate-100' : 'bg-studio-light text-slate-900'
      }`}
    >
      {/* Top Header Bar - Modern Dark Studio */}
      <header
        className={`sticky top-0 z-30 px-6 py-3 border-b glass-panel transition-colors ${
          isDark
            ? 'bg-[#08090d]/85 border-white/[0.07]'
            : 'bg-white/85 border-slate-200/80'
        }`}
      >
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          {/* Brand Logo & Name */}
          <div className="flex items-center gap-3">
            <HdrLogo size={34} active={status.is_hdr_active} />

            <div className="flex items-center gap-2.5">
              <span className="font-bold text-[15px] tracking-tight">
                HDR Auto-Switch
              </span>

              {/* Status Pill Badge */}
              <div
                className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium transition-all ${
                  status.is_hdr_active
                    ? 'bg-rose-500/15 text-rose-300 border border-rose-500/30 shadow-[0_0_12px_rgba(244,63,94,0.25)]'
                    : isDark
                    ? 'bg-white/[0.05] text-slate-400 border border-white/[0.08]'
                    : 'bg-slate-100 text-slate-600 border border-slate-200'
                }`}
              >
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    status.is_hdr_active
                      ? 'bg-rose-400 animate-pulse'
                      : 'bg-slate-400'
                  }`}
                />
                <span>{status.is_hdr_active ? 'HDR Aktivní' : 'SDR Standby'}</span>
              </div>
            </div>
          </div>

          {/* Clean Segmented Navigation Bar */}
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
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-all duration-150 ${
                    isActive
                      ? isDark
                        ? 'bg-white/10 text-white shadow-sm border border-white/10'
                        : 'bg-white text-slate-900 shadow-sm border border-slate-200'
                      : isDark
                      ? 'text-slate-400 hover:text-white hover:bg-white/[0.04] border border-transparent'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-200/50 border border-transparent'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Right Controls: Telemetry + Theme Switch */}
          <div className="flex items-center gap-3">
            <div
              className={`hidden md:flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-[11px] font-mono ${
                isDark
                  ? 'bg-white/[0.02] border-white/[0.06] text-slate-400'
                  : 'bg-slate-50 border-slate-200 text-slate-600'
              }`}
            >
              <Cpu className="w-3 h-3 text-emerald-400" />
              <span>0.0% CPU</span>
            </div>

            <button
              onClick={() => setIsDark(!isDark)}
              className={`p-2 rounded-xl border transition-all cursor-pointer ${
                isDark
                  ? 'border-white/[0.08] bg-white/[0.03] text-amber-400 hover:bg-white/[0.08]'
                  : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-100 shadow-sm'
              }`}
              title={isDark ? 'Přepnout na světlý režim' : 'Přepnout na tmavý režim'}
            >
              {isDark ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
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
        className={`px-6 py-2.5 border-t text-[11px] font-mono transition-colors glass-panel ${
          isDark
            ? 'bg-[#08090d]/85 border-white/[0.06] text-slate-400'
            : 'bg-white/80 border-slate-200 text-slate-600'
        }`}
      >
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <span className="relative flex h-2 w-2">
              <span
                className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${
                  status.is_hdr_active ? 'bg-rose-400' : 'bg-emerald-400'
                }`}
              />
              <span
                className={`relative inline-flex rounded-full h-2 w-2 ${
                  status.is_hdr_active ? 'bg-rose-500' : 'bg-emerald-500'
                }`}
              />
            </span>
            <span className="font-medium">
              {status.is_hdr_active
                ? `HDR Aktivní • ${status.current_app_name || 'Ruční přepnutí'}`
                : 'SDR Standby • Sledování procesů aktivní'}
            </span>
          </div>

          <div className="flex items-center gap-3 text-[11px] text-slate-500">
            <span>{config.apps.filter((a) => a.enabled).length} sledovaných her</span>
            <span>•</span>
            <span>WinEventHook na pozadí</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
