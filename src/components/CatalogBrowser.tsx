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
      await invoke('sync_database');
      await fetchCatalog();
      setMessage('Databáze byla úspěšně aktualizována.');
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      console.error('Sync failed:', err);
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
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
            <CheckCircle2 className="w-3 h-3 text-emerald-400" /> Nativní podpora
          </span>
        );
      case 'limited':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-teal-500/10 text-teal-400 border border-teal-500/20 font-medium">
            <Sparkles className="w-3 h-3 text-teal-400" /> Omezená podpora
          </span>
        );
      case 'always_on':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-lime-500/10 text-lime-400 border border-lime-500/20 font-medium">
            <Lock className="w-3 h-3 text-lime-400" /> Vždy zapnuto
          </span>
        );
      case 'manual_fix':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-blue-500/10 text-blue-400 border border-blue-500/20 font-medium">
            <Wrench className="w-3 h-3 text-blue-400" /> Vyžaduje úpravu
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-medium">
            <Zap className="w-3 h-3 text-amber-400" /> Windows Auto HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-md bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 font-medium">
            <Film className="w-3 h-3 text-cyan-400" /> Přehrávač
          </span>
        );
      default:
        return null;
    }
  };

  return (
    <div className="space-y-4 animate-fadeIn">
      {/* Search and sync header */}
      <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat v PCGamingWiki databázi (např. Witcher, Elden Ring, Skyrim)..."
            className={`w-full pl-9 pr-4 py-2 text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-slate-900/60 border-white/10 focus:border-cyan-500 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400'
            }`}
          />
        </div>

        <button
          onClick={handleSync}
          disabled={syncing}
          className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-semibold cursor-pointer transition-all ${
            isDark
              ? 'border-white/10 hover:bg-white/5 text-slate-300'
              : 'border-slate-300 hover:bg-slate-100 text-slate-700'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin' : ''}`} />
          Aktualizovat z webu
        </button>
      </div>

      {message && (
        <div className="p-3 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 text-xs">
          {message}
        </div>
      )}

      {/* PCGamingWiki Support Legend / Filter bar */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
        {[
          { id: 'all', label: `Vše (${catalog.length})` },
          { id: 'native', label: `Nativní HDR (${catalog.filter((c) => c.support_tier === 'native').length})` },
          { id: 'limited', label: `Omezené (${catalog.filter((c) => c.support_tier === 'limited').length})` },
          { id: 'always_on', label: `Vždy zapnuto (${catalog.filter((c) => c.support_tier === 'always_on').length})` },
          { id: 'manual_fix', label: `Vyžaduje fix (${catalog.filter((c) => c.support_tier === 'manual_fix').length})` },
          { id: 'autohdr', label: `Auto HDR (${catalog.filter((c) => c.support_tier === 'autohdr').length})` },
          { id: 'media', label: `Média (${catalog.filter((c) => c.support_tier === 'media').length})` },
        ].map((tier) => (
          <button
            key={tier.id}
            onClick={() => setSelectedTier(tier.id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-all whitespace-nowrap ${
              selectedTier === tier.id
                ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 font-semibold'
                : isDark
                ? 'bg-slate-900/40 text-slate-400 hover:text-white border border-transparent'
                : 'bg-white/60 text-slate-600 hover:text-slate-900 border border-slate-200'
            }`}
          >
            {tier.label}
          </button>
        ))}
      </div>

      {/* Catalog items list */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-slate-900/60 border-white/10' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="divide-y divide-white/5 max-h-[480px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-sm">
              {loading ? 'Načítám katalog her...' : 'Nebyly nalezeny žádné hry.'}
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
                    isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/50'
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4 className="text-sm font-semibold truncate">{item.name}</h4>
                      {getTierBadge(item.support_tier)}
                    </div>
                    <div className="flex items-center gap-2 mt-0.5">
                      <span className="text-xs text-slate-400 font-mono">
                        {item.exe_name}
                      </span>
                      {item.notes && (
                        <>
                          <span className="text-slate-600">•</span>
                          <span className="text-xs text-slate-400 truncate">
                            {item.notes}
                          </span>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="shrink-0">
                    {isAdded ? (
                      <div className="flex items-center gap-2">
                        <span className="inline-flex items-center gap-1 text-xs px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                          <Check className="w-3.5 h-3.5" /> V mých aplikacích
                        </span>
                        <button
                          onClick={() => handleRemoveGame(item.exe_name)}
                          className="text-xs text-slate-500 hover:text-rose-400 cursor-pointer px-1 py-1"
                          title="Odebrat z mých aplikací"
                        >
                          ✕
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => handleAddGame(item)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-700 text-white text-xs font-semibold shadow-sm cursor-pointer transition-all"
                      >
                        <Plus className="w-3.5 h-3.5" /> Přidat do mých her
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
