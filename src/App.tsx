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
  Activity,
  Terminal,
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
          ? 'bg-[#06080d] text-slate-100 bg-tech-grid'
          : 'bg-[#f6f8fb] text-slate-900 bg-tech-grid-light'
      }`}
      style={{
        background: isDark
          ? 'radial-gradient(circle at 50% 0%, rgba(14, 28, 55, 0.45) 0%, rgba(6, 8, 13, 0.98) 75%)'
          : 'radial-gradient(circle at 50% 0%, rgba(224, 238, 255, 0.6) 0%, rgba(246, 248, 251, 0.98) 75%)',
      }}
    >
      {/* Top Header Bar - Geometric Studio Aesthetic */}
      <header
        className={`sticky top-0 z-30 px-6 py-3 border-b backdrop-blur-2xl transition-colors glass-panel ${
          isDark
            ? 'bg-[#07090e]/80 border-white/[0.08]'
            : 'bg-white/85 border-slate-200'
        }`}
      >
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          {/* Brand & Technical Readout */}
          <div className="flex items-center gap-3.5">
            <HdrLogo size={36} active={status.is_hdr_active} />

            <div>
              <div className="flex items-center gap-2">
                <h1 className="font-extrabold text-[15px] tracking-tight flex items-center gap-1.5">
                  HDR AUTO-SWITCH
                </h1>
                <span
                  className={`text-[9px] font-mono font-bold px-1.5 py-0.5 rounded tracking-widest uppercase transition-all ${
                    status.is_hdr_active
                      ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30 shadow-[0_0_12px_rgba(244,63,94,0.3)]'
                      : isDark
                      ? 'bg-cyan-950/40 text-cyan-400 border border-cyan-500/20'
                      : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  {status.is_hdr_active ? 'HDR ACTIVE' : 'SDR STANDBY'}
                </span>
              </div>
              <div className="flex items-center gap-2 text-[10px] font-mono text-slate-400">
                <span>WIN32 HOOK // 0.0% CPU</span>
                <span className="text-slate-600">•</span>
                <span className="text-cyan-400/80">LATENCY // 0ms</span>
              </div>
            </div>
          </div>

          {/* Navigation Tabs - Geometric Pill style */}
          <nav className="flex items-center gap-1 bg-white/[0.03] p-1 rounded-xl border border-white/[0.05]">
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
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-all ${
                    isActive
                      ? isDark
                        ? 'bg-white/10 text-white shadow-[0_0_15px_rgba(255,255,255,0.08)] border border-white/15'
                        : 'bg-white text-slate-900 shadow-sm border border-slate-300'
                      : isDark
                      ? 'text-slate-400 hover:text-white hover:bg-white/5 border border-transparent'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-200/60 border border-transparent'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Controls: Telemetry readout + Theme Switch */}
          <div className="flex items-center gap-2.5">
            <div className="hidden lg:flex items-center gap-2 px-2.5 py-1 rounded-lg border border-white/5 bg-white/[0.02] text-[10px] font-mono text-slate-400">
              <Activity className="w-3 h-3 text-emerald-400 animate-pulse" />
              <span>LIVE OS MONITOR</span>
            </div>

            <button
              onClick={() => setIsDark(!isDark)}
              className={`p-2 rounded-xl border transition-all cursor-pointer ${
                isDark
                  ? 'border-white/10 bg-slate-900/80 text-amber-400 hover:bg-slate-800'
                  : 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'
              }`}
              title={isDark ? 'Světlý režim' : 'Tmavý režim'}
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

      {/* Status Bar Footer - Studio Telemetry */}
      <footer
        className={`px-6 py-2 border-t text-[11px] font-mono transition-colors backdrop-blur-md glass-panel ${
          isDark
            ? 'bg-[#07090e]/80 border-white/[0.05] text-slate-400'
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
                ? `HDR ACTIVE // ${status.current_app_name || 'MANUAL TRIGGER'}`
                : 'SDR MODE // LISTENING FOR HDR WINDOWS'}
            </span>
          </div>

          <div className="flex items-center gap-4 text-[10px] text-slate-500">
            <span>MONITORED // {config.apps.filter((a) => a.enabled).length} GAMES</span>
            <span>•</span>
            <span>TRAY RESIDENT // ACTIVE</span>
            <span>•</span>
            <span className="text-emerald-400/90 flex items-center gap-1">
              <Terminal className="w-3 h-3" />
              0.0% CPU (EVENT HOOK)
            </span>
          </div>
        </div>
      </footer>
    </div>
  );
}
