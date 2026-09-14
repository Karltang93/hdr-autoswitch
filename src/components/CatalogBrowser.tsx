import React, { useState, useEffect } from 'react';
import { CatalogEntry, AppConfig, HdrApp, SupportTier } from '../types';
import { invoke } from '@tauri-apps/api/core';
import {
  Search,
  CheckCircle2,
  Plus,
  RefreshCw,
  Check,
  Sparkles,
  Lock,
  Wrench,
  Film,
  Zap,
} from 'lucide-react';

interface CatalogBrowserProps {
  config: AppConfig;
  onUpdateConfig: (newConfig: AppConfig) => void;
  isDark: boolean;
}

export const CatalogBrowser: React.FC<CatalogBrowserProps> = ({
  config,
  onUpdateConfig,
  isDark,
}) => {
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [selectedTier, setSelectedTier] = useState<string>('all');
  const [syncing, setSyncing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const fetchCatalog = async () => {
    setLoading(true);
    try {
      const entries: CatalogEntry[] = await invoke('get_catalog');
      setCatalog(entries);
    } catch (err) {
      console.error('Failed to get catalog:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCatalog();
  }, []);

  const handleAddGame = async (entry: CatalogEntry) => {
    const newApp: HdrApp = {
      name: entry.name,
      exe_name: entry.exe_name.toLowerCase(),
      enabled: true,
      hdr_type: entry.hdr_type,
    };

    try {
      await invoke('add_custom_app', { app: newApp });
      const refreshed: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshed);
    } catch (err) {
      console.error('Failed to add app from catalog:', err);
    }
  };

  const handleRemoveGame = async (exeName: string) => {
    try {
      await invoke('remove_app', { exeName });
      const refreshed: AppConfig = await invoke('get_config');
      onUpdateConfig(refreshed);
    } catch (err) {
      console.error('Failed to remove app:', err);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      const count: number = await invoke('sync_database');
      await fetchCatalog();
      setMessage(`Databáze úspěšně synchronizována (${count} titulů v katalogu).`);
      setTimeout(() => setMessage(null), 4000);
    } catch (err) {
      console.error('Sync failed:', err);
      setMessage('Chyba při aktualizaci databáze.');
      setTimeout(() => setMessage(null), 4000);
    } finally {
      setSyncing(false);
    }
  };

  const filtered = catalog.filter((item) => {
    const matchesSearch =
      item.name.toLowerCase().includes(search.toLowerCase()) ||
      item.exe_name.toLowerCase().includes(search.toLowerCase());

    const matchesTier =
      selectedTier === 'all' ? true : item.support_tier === selectedTier;

    return matchesSearch && matchesTier;
  });

  const getTierBadge = (tier: SupportTier) => {
    switch (tier) {
      case 'native':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
            <CheckCircle2 className="w-3 h-3 text-emerald-400" /> Nativní HDR
          </span>
        );
      case 'limited':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-teal-500/10 text-teal-300 border border-teal-500/20 font-medium">
            <Sparkles className="w-3 h-3 text-teal-400" /> Omezené
          </span>
        );
      case 'always_on':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-lime-500/10 text-lime-300 border border-lime-500/20 font-medium">
            <Lock className="w-3 h-3 text-lime-400" /> Vždy zapnuto
          </span>
        );
      case 'manual_fix':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-blue-500/10 text-blue-300 border border-blue-500/20 font-medium">
            <Wrench className="w-3 h-3 text-blue-400" /> Vyžaduje mod/fix
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-amber-500/10 text-amber-300 border border-amber-500/20 font-medium">
            <Zap className="w-3 h-3 text-amber-400" /> Auto HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.2 rounded-md bg-cyan-500/10 text-cyan-300 border border-cyan-500/20 font-medium">
            <Film className="w-3 h-3 text-cyan-400" /> Přehrávač médií
          </span>
        );
      default:
        return null;
    }
  };

  const countByTier = (tier: string) => {
    if (tier === 'all') return catalog.length;
    return catalog.filter((i) => i.support_tier === tier).length;
  };

  const tiers = [
    { id: 'all', label: `Vše (${countByTier('all')})` },
    { id: 'native', label: `Nativní HDR (${countByTier('native')})` },
    { id: 'autohdr', label: `Auto HDR (${countByTier('autohdr')})` },
    { id: 'manual_fix', label: `Vyžaduje fix (${countByTier('manual_fix')})` },
    { id: 'limited', label: `Omezené (${countByTier('limited')})` },
    { id: 'always_on', label: `Vždy zapnuto (${countByTier('always_on')})` },
    { id: 'media', label: `Média (${countByTier('media')})` },
  ];

  return (
    <div className="space-y-4">
      {/* Search and Online Sync Controls */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat v PCGamingWiki databázi (např. Cyberpunk, Witcher, Elden Ring, Forza)..."
            className={`w-full pl-9 pr-4 py-2 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0c0f18]/80 border-white/[0.08] focus:border-sky-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-sky-500 text-slate-900 placeholder-slate-400 shadow-sm'
            }`}
          />
        </div>

        <button
          onClick={handleSync}
          disabled={syncing}
          className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-medium cursor-pointer transition-all ${
            isDark
              ? 'border-white/[0.08] hover:bg-white/[0.04] text-slate-300'
              : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-sm'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin text-sky-400' : 'text-slate-400'}`} />
          <span>{syncing ? 'Aktualizuji...' : 'Aktualizovat z webu'}</span>
        </button>
      </div>

      {message && (
        <div className="p-3 rounded-xl bg-sky-500/10 border border-sky-500/20 text-sky-300 text-xs flex items-center gap-2">
          <Sparkles className="w-4 h-4 shrink-0 text-sky-400" />
          <span>{message}</span>
        </div>
      )}

      {/* Filter Tier Tabs */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1 scrollbar-none">
        {tiers.map((tier) => (
          <button
            key={tier.id}
            onClick={() => setSelectedTier(tier.id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-all whitespace-nowrap ${
              selectedTier === tier.id
                ? 'bg-sky-500/15 text-sky-300 border border-sky-500/30 font-semibold'
                : isDark
                ? 'bg-[#0c0f18]/60 text-slate-400 hover:text-white border border-white/[0.05]'
                : 'bg-white text-slate-600 hover:text-slate-900 border border-slate-200'
            }`}
          >
            {tier.label}
          </button>
        ))}
      </div>

      {/* Catalog items list */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-[#0c0f18]/80 border-white/[0.07]' : 'bg-white/80 border-slate-200 shadow-sm'
        }`}
      >
        <div className="divide-y divide-white/[0.04] max-h-[500px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-10 text-center text-slate-400 text-xs">
              {loading ? 'Načítám databázi her...' : 'Nebyly nalezeny žádné hry odpovídající filtru.'}
            </div>
          ) : (
            filtered.map((item) => {
              const exeLower = item.exe_name.toLowerCase();
              const isAdded = config.apps.some(
                (a) => a.exe_name.toLowerCase() === exeLower
              );

              return (
                <div
                  key={`${item.name}-${item.exe_name}`}
                  className={`p-3.5 flex items-center justify-between gap-4 transition-colors ${
                    isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/70'
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4 className="text-sm font-semibold truncate text-slate-100">{item.name}</h4>
                      {getTierBadge(item.support_tier)}
                    </div>
                    <div className="flex items-center gap-2 mt-0.5">
                      <span className="text-xs text-slate-400 font-mono">
                        {item.exe_name}
                      </span>
                      {item.notes && (
                        <>
                          <span className="text-slate-600">•</span>
                          <span className="text-xs text-slate-400 truncate max-w-[320px]">
                            {item.notes}
                          </span>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="shrink-0">
                    {isAdded ? (
                      <div className="flex items-center gap-1.5">
                        <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/25 font-medium">
                          <Check className="w-3.5 h-3.5" /> Sledováno
                        </span>
                        <button
                          onClick={() => handleRemoveGame(item.exe_name)}
                          className="p-1 rounded text-slate-500 hover:text-rose-400 cursor-pointer transition-colors"
                          title="Odebrat z mých her"
                        >
                          ✕
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => handleAddGame(item)}
                        className="flex items-center gap-1 px-3 py-1 rounded-lg border border-white/[0.08] hover:border-sky-400/40 hover:bg-sky-500/10 text-slate-300 hover:text-sky-200 text-xs font-medium cursor-pointer transition-all duration-150"
                      >
                        <Plus className="w-3.5 h-3.5" />
                        <span>Přidat</span>
                      </button>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
