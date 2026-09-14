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
  BookOpen,
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
      setMessage(`Databáze byla úspěšně synchronizována (${count} her a aplikací).`);
      setTimeout(() => setMessage(null), 4000);
    } catch (err) {
      console.error('Sync failed:', err);
      setMessage('Chyba při aktualizaci z webu.');
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
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 font-semibold">
            <CheckCircle2 className="w-3 h-3 text-emerald-400" /> NATIVNÍ HDR
          </span>
        );
      case 'limited':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-teal-500/15 text-teal-300 border border-teal-500/30 font-semibold">
            <Sparkles className="w-3 h-3 text-teal-400" /> OMEZENÉ
          </span>
        );
      case 'always_on':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-lime-500/15 text-lime-300 border border-lime-500/30 font-semibold">
            <Lock className="w-3 h-3 text-lime-400" /> VŽDY ZAPNUTO
          </span>
        );
      case 'manual_fix':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-blue-500/15 text-blue-300 border border-blue-500/30 font-semibold">
            <Wrench className="w-3 h-3 text-blue-400" /> VYŽADUJE MOD/FIX
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-amber-500/15 text-amber-300 border border-amber-500/30 font-semibold">
            <Zap className="w-3 h-3 text-amber-400" /> AUTO HDR
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-cyan-500/15 text-cyan-300 border border-cyan-500/30 font-semibold">
            <Film className="w-3 h-3 text-cyan-400" /> PŘEHRÁVAČ
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
            placeholder="Hledat v PCGamingWiki databázi (např. Cyberpunk, Witcher, Elden Ring, Skyrim)..."
            className={`w-full pl-9 pr-4 py-2 text-xs md:text-sm rounded-xl border transition-all ${
              isDark
                ? 'bg-[#0b0f19]/80 border-white/[0.08] focus:border-cyan-500/50 text-white placeholder-slate-500'
                : 'bg-white border-slate-200 focus:border-cyan-500 text-slate-900 placeholder-slate-400'
            }`}
          />
        </div>

        <button
          onClick={handleSync}
          disabled={syncing}
          className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl border text-xs font-mono font-bold cursor-pointer transition-all shadow-sm ${
            isDark
              ? 'border-white/10 hover:bg-white/5 text-slate-300 hover:border-cyan-500/30'
              : 'border-slate-300 hover:bg-slate-100 text-slate-700'
          } disabled:opacity-50`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin text-cyan-400' : 'text-slate-400'}`} />
          <span>{syncing ? 'SYNCHRONIZUJI...' : 'AKTUALIZOVAT Z WEBU'}</span>
        </button>
      </div>

      {message && (
        <div className="p-3 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-300 text-xs font-mono animate-fadeIn flex items-center gap-2">
          <BookOpen className="w-4 h-4 text-cyan-400 shrink-0" />
          <span>{message}</span>
        </div>
      )}

      {/* PCGamingWiki Support Legend / Filter bar with exact dynamic counts */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
        {[
          { id: 'all', label: `Vše (${catalog.length})` },
          { id: 'native', label: `Nativní HDR (${catalog.filter((c) => c.support_tier === 'native').length})` },
          { id: 'autohdr', label: `Auto HDR (${catalog.filter((c) => c.support_tier === 'autohdr').length})` },
          { id: 'manual_fix', label: `Vyžaduje fix (${catalog.filter((c) => c.support_tier === 'manual_fix').length})` },
          { id: 'limited', label: `Omezené (${catalog.filter((c) => c.support_tier === 'limited').length})` },
          { id: 'always_on', label: `Vždy zapnuto (${catalog.filter((c) => c.support_tier === 'always_on').length})` },
          { id: 'media', label: `Média (${catalog.filter((c) => c.support_tier === 'media').length})` },
        ].map((tier) => (
          <button
            key={tier.id}
            onClick={() => setSelectedTier(tier.id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-mono font-medium cursor-pointer transition-all whitespace-nowrap ${
              selectedTier === tier.id
                ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-bold shadow-[0_0_12px_rgba(6,182,212,0.2)]'
                : isDark
                ? 'bg-[#090d16]/70 text-slate-400 hover:text-white border border-white/[0.05]'
                : 'bg-white/60 text-slate-600 hover:text-slate-900 border border-slate-200'
            }`}
          >
            {tier.label}
          </button>
        ))}
      </div>

      {/* Catalog items list with geometric corner brackets */}
      <div
        className={`rounded-2xl border overflow-hidden glass-panel corner-brackets ${
          isDark ? 'bg-[#090d16]/80 border-white/[0.08]' : 'bg-white/70 border-slate-200'
        }`}
      >
        <div className="divide-y divide-white/[0.04] max-h-[480px] overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="p-8 text-center text-slate-500 text-xs font-mono">
              {loading ? 'NAČÍTÁM KATALOG PCGAMINGWIKI...' : 'NEBYLY NALEZENY ŽÁDNÉ HRY.'}
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
                    isDark ? 'hover:bg-white/[0.02]' : 'hover:bg-slate-50/60'
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
                        <span className="inline-flex items-center gap-1 text-xs font-mono px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 font-medium">
                          <Check className="w-3.5 h-3.5" /> SLEDOVÁNO
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
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white text-xs font-mono font-bold shadow-sm cursor-pointer transition-all hover:scale-105"
                      >
                        <Plus className="w-3.5 h-3.5" /> PŘIDAT DO MÝCH HER
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
