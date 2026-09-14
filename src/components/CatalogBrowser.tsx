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
      setMessage(`Databáze byla úspěšně synchronizována z webu (${count} titulů).`);
      setTimeout(() => setMessage(null), 4000);
    } catch (err) {
      console.error('Sync failed:', err);
      setMessage('Chyba při stahování databáze z PCGamingWiki.');
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
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-bold uppercase tracking-wider">
            <CheckCircle2 className="w-3 h-3 text-emerald-400" /> Nativní HDR
          </span>
        );
      case 'limited':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-teal-500/20 text-teal-300 border border-teal-500/30 font-bold uppercase tracking-wider">
            <Sparkles className="w-3 h-3 text-teal-400" /> Omezené
          </span>
        );
      case 'always_on':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-lime-500/20 text-lime-300 border border-lime-500/30 font-bold uppercase tracking-wider">
            <Lock className="w-3 h-3 text-lime-400" /> Vždy zapnuto
          </span>
        );
      case 'manual_fix':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-blue-500/20 text-blue-300 border border-blue-500/30 font-bold uppercase tracking-wider">
            <Wrench className="w-3 h-3 text-blue-400" /> Vyžaduje mod/fix
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-amber-500/20 text-amber-300 border border-amber-500/30 font-bold uppercase tracking-wider">
            <Zap className="w-3 h-3 text-amber-400" /> Auto HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2.5 py-0.5 rounded-md bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold uppercase tracking-wider">
            <Film className="w-3 h-3 text-cyan-400" /> Média
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
    <div className="space-y-5">
      {/* Header with Search and Online Sync */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-black tracking-tight text-white flex items-center gap-2">
            <span>Databáze her</span>
            <span className="text-xs font-mono font-normal px-2.5 py-0.5 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
              {catalog.length} her a aplikací
            </span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Kompletní katalog her s ověřenou podporou Nativního HDR a Windows Auto HDR z PCGamingWiki.
          </p>
        </div>

        <button
          onClick={handleSync}
          disabled={syncing}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl border text-xs font-bold cursor-pointer transition-all ${
            isDark
              ? 'border-white/10 hover:border-cyan-500/40 hover:bg-white/[0.04] text-slate-200 shadow-sm'
              : 'border-slate-200 hover:bg-slate-100 text-slate-700 shadow-xs'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin text-cyan-400' : 'text-cyan-400'}`} />
          <span>{syncing ? 'Aktualizuji databázi...' : 'Aktualizovat z webu'}</span>
        </button>
      </div>

      {message && (
        <div className="p-3.5 rounded-2xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-300 text-xs flex items-center gap-2.5 shadow-sm">
          <Sparkles className="w-4 h-4 shrink-0 text-cyan-400" />
          <span className="font-medium">{message}</span>
        </div>
      )}

      {/* Search and Category Filter Toolbar */}
      <div className="space-y-3">
        <div className="relative">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-slate-400" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Hledat v databázi (např. Cyberpunk, Witcher, Elden Ring, Forza, Battlefield)..."
            className={`w-full pl-9 pr-4 py-2.5 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0e1322]/80 border-white/[0.08] focus:border-cyan-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400 shadow-xs'
            }`}
          />
        </div>

        {/* Filter Pills */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1 scrollbar-none">
          {tiers.map((tier) => (
            <button
              key={tier.id}
              onClick={() => setSelectedTier(tier.id)}
              className={`px-3.5 py-1.5 rounded-xl text-xs font-bold cursor-pointer transition-all whitespace-nowrap ${
                selectedTier === tier.id
                  ? 'bg-cyan-500 text-slate-950 font-extrabold shadow-md neon-glow-cyan'
                  : isDark
                  ? 'bg-[#0e1322]/60 text-slate-400 hover:text-white border border-white/[0.06] hover:border-cyan-500/30'
                  : 'bg-white text-slate-600 hover:text-slate-900 border border-slate-200'
              }`}
            >
              {tier.label}
            </button>
          ))}
        </div>
      </div>

      {/* Catalog items list */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel ${
          isDark ? 'bg-[#0f1422]/80 border-white/[0.08]' : 'bg-white/90 border-slate-200 shadow-md'
        }`}
      >
        <div className="divide-y divide-white/[0.05] max-h-[520px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-16 text-center text-slate-400 text-xs font-mono">
              {loading ? 'NAČÍTÁM DATABÁZI HER...' : 'NEBYLY NALEZENY ŽÁDNÉ HRY ODPOVÍDAJÍCÍ FILTRU.'}
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
                    isDark ? 'hover:bg-white/[0.03]' : 'hover:bg-slate-50/80'
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2.5 flex-wrap">
                      <h4 className="text-sm font-extrabold truncate text-slate-100">{item.name}</h4>
                      {getTierBadge(item.support_tier)}
                    </div>
                    <div className="flex items-center gap-2 mt-1">
                      <span className="text-xs text-slate-400 font-mono">
                        {item.exe_name}
                      </span>
                      {item.notes && (
                        <>
                          <span className="text-slate-600">•</span>
                          <span className="text-xs text-slate-400 truncate max-w-[340px]">
                            {item.notes}
                          </span>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="shrink-0">
                    {isAdded ? (
                      <div className="flex items-center gap-2">
                        <span className="inline-flex items-center gap-1.5 text-xs font-bold px-3 py-1.5 rounded-xl bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 neon-glow-emerald">
                          <Check className="w-4 h-4" /> SLEDOVÁNO
                        </span>
                        <button
                          onClick={() => handleRemoveGame(item.exe_name)}
                          className="p-1.5 rounded-lg text-slate-400 hover:text-rose-400 hover:bg-rose-500/15 cursor-pointer transition-colors"
                          title="Odebrat z mých her"
                        >
                          ✕
                        </button>
                      </div>
                    ) : (
                      /* Restored High-Energy Neon Button */
                      <button
                        onClick={() => handleAddGame(item)}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 hover:text-white font-bold text-xs shadow-md neon-glow-cyan cursor-pointer transition-all duration-150 hover:scale-105 active:scale-95"
                      >
                        <Plus className="w-4 h-4 fill-current" />
                        <span>PŘIDAT DO MÝCH HER</span>
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
