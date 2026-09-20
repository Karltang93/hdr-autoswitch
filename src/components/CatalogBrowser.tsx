import React, { useState, useEffect } from 'react';
import { CatalogEntry, AppConfig, HdrApp, SupportTier } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { configClient } from '../useConfig';
import { findTrackedApp } from '../libraryState';
import { catalogNotes } from '../catalogNotes';
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
import { GlitchButton } from './GlitchButton';
import { GlitchText } from './GlitchText';
import { useI18n } from '../i18n';

interface CatalogBrowserProps {
  config: AppConfig;
  isDark: boolean;
}

export const CatalogBrowser: React.FC<CatalogBrowserProps> = ({
  config,
}) => {
  const { t, lang } = useI18n();
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
      steam_id: entry.steam_id,
      alternate_exes: entry.alternate_exes,
    };

    try {
      await configClient.mutate('add_custom_app', { app: newApp });
    } catch (err) {
      configClient.reportError(err);
    }
  };

  const handleRemoveGame = async (exeName: string) => {
    try {
      await configClient.mutate('remove_app', { exeName });
    } catch (err) {
      configClient.reportError(err);
    }
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      const count: number = await invoke('sync_database');
      await fetchCatalog();
      setMessage(t.catalogSyncSuccess(count));
      setTimeout(() => setMessage(null), 4000);
    } catch (err) {
      console.error('Sync failed:', err);
      configClient.reportError(err);
      setMessage(t.catalogSyncError);
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
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-cyan-950/80 text-[#5accf5] border border-[#5accf5]/40 font-bold uppercase tracking-wider">
            <CheckCircle2 className="w-3 h-3 text-[#5accf5]" /> {t.catalogTierNative}
          </span>
        );
      case 'limited':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-teal-950/80 text-teal-300 border border-teal-500/40 font-bold uppercase tracking-wider">
            <Sparkles className="w-3 h-3 text-teal-400" /> {t.catalogTierLimited}
          </span>
        );
      case 'always_on':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-purple-950/80 text-purple-300 border border-purple-500/40 font-bold uppercase tracking-wider">
            <Lock className="w-3 h-3 text-purple-400" /> {t.catalogTierAlwaysOn}
          </span>
        );
      case 'manual_fix':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-amber-950/80 text-amber-300 border border-amber-500/40 font-bold uppercase tracking-wider">
            <Wrench className="w-3 h-3 text-amber-400" /> {t.catalogTierMod}
          </span>
        );
      case 'autohdr':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-rose-950/80 text-[#f55a6b] border border-[#f55a6b]/40 font-bold uppercase tracking-wider">
            <Zap className="w-3 h-3 text-[#f55a6b]" /> {t.catalogTierAutoHdr}
          </span>
        );
      case 'media':
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-blue-950/80 text-blue-300 border border-blue-500/40 font-bold uppercase tracking-wider">
            <Film className="w-3 h-3 text-blue-400" /> {t.catalogTierMedia}
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 bg-slate-900 text-slate-300 border border-slate-700 font-bold uppercase tracking-wider">
            {t.catalogTierCustom}
          </span>
        );
    }
  };

  const countForTier = (tier: string) => {
    if (tier === 'all') return catalog.length;
    return catalog.filter((i) => i.support_tier === tier).length;
  };

  return (
    <div className="space-y-5 font-mono">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2.5">
            <h2 className="glitch-title-bar px-2.5 py-0.5 text-xs font-bold tracking-wider inline-block">
              {t.catalogTitle}
            </h2>
            <span className="text-xs px-2 py-0.5 border border-[#5accf5]/40 text-[#5accf5] bg-[#140e10]">
              {t.catalogArchiveCount(catalog.length)}
            </span>
          </div>
          <p className="text-xs text-[#8a7f81] mt-1">
            {t.catalogSubtitle}
          </p>
        </div>

        <GlitchButton
          label={syncing ? t.catalogSyncingBtn : t.catalogSyncBtn}
          variant="outline"
          size="sm"
          disabled={syncing}
          icon={<RefreshCw className={`w-3.5 h-3.5 text-[#5accf5] ${syncing ? 'animate-spin' : ''}`} />}
          onClick={handleSync}
        />
      </div>

      {message && (
        <div className="p-3 border border-[#5accf5]/40 bg-[#120e10] text-[#5accf5] text-xs flex items-center gap-2.5">
          <Sparkles className="w-4 h-4 shrink-0 text-[#5accf5]" />
          <span>&gt; {message}</span>
        </div>
      )}

      {/* Filter and Search Bar */}
      <div className="space-y-3">
        <div className="relative">
          <Search className="w-4 h-4 absolute left-3.5 top-1/2 -translate-y-1/2 text-[#8a7f81]" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t.catalogSearchPlaceholder}
            className="w-full pl-9 pr-4 py-2 text-xs border border-[#f55a6b]/30 bg-[#120d0e] focus:border-[#f55a6b] text-white placeholder-[#8a7f81] focus:outline-none transition-all"
          />
        </div>

        {/* Tier Filter Tabs */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
          {[
            { id: 'all', label: t.catalogTabAll(countForTier('all')) },
            { id: 'native', label: t.catalogTabNative(countForTier('native')) },
            { id: 'autohdr', label: t.catalogTabAutoHdr(countForTier('autohdr')) },
            { id: 'limited', label: t.catalogTabLimited(countForTier('limited')) },
            { id: 'manual_fix', label: t.catalogTabMod(countForTier('manual_fix')) },
            { id: 'always_on', label: t.catalogTabAlwaysOn(countForTier('always_on')) },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setSelectedTier(tab.id)}
              className={`px-3 py-1 text-xs uppercase font-bold cursor-pointer transition-all border ${
                selectedTier === tab.id
                  ? 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b] neon-glow-coral'
                  : 'bg-[#120d0e] text-[#8a7f81] border-[#f55a6b]/20 hover:border-[#f55a6b]/50 hover:text-white'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Games Catalog List */}
      {loading ? (
        <div className="p-12 text-center border border-[#f55a6b]/20 bg-[#120d0e] text-[#5accf5] text-xs">
          {t.catalogLoading}
        </div>
      ) : filtered.length === 0 ? (
        <div className="p-12 text-center border border-[#f55a6b]/20 bg-[#120d0e] text-[#8a7f81] text-xs">
          {t.catalogEmpty}
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map((item) => {
            const tracked = findTrackedApp(config.apps, item);

            return (
              <div
                key={item.exe_name}
                className={`p-3 border transition-all flex items-center justify-between gap-4 relative ${
                  tracked
                    ? 'bg-[#180e10] border-[#f55a6b]/50'
                    : 'bg-[#120d0e] border-[#f55a6b]/20 hover:border-[#f55a6b]/60'
                }`}
              >
                <div className="absolute inset-0 scanlines-overlay opacity-10 pointer-events-none" />

                <div className="space-y-1 min-w-0 relative z-10">
                  <div className="flex items-center gap-2.5 flex-wrap">
                    <span className="font-bold text-sm text-white truncate">
                      <GlitchText text={item.name} scrambleOnHover={true} />
                    </span>
                    {getTierBadge(item.support_tier)}
                  </div>

                  <div className="flex items-center gap-2 text-xs text-[#8a7f81]">
                    <span className="text-[#5accf5] font-mono">[{item.exe_name}]</span>
                    {item.notes && (
                      <>
                        <span>•</span>
                        <span className="truncate max-w-[400px]">{catalogNotes(item.notes, lang)}</span>
                      </>
                    )}
                  </div>
                </div>

                <div className="shrink-0 relative z-10">
                  {tracked ? (
                    <GlitchButton
                      label={t.catalogRemoveBtn}
                      variant="outline"
                      size="sm"
                      icon={<Check className="w-3.5 h-3.5 text-emerald-400" />}
                      onClick={() => handleRemoveGame(tracked.exe_name)}
                    />
                  ) : (
                    <GlitchButton
                      label={t.catalogAddBtn}
                      variant="primary"
                      size="sm"
                      icon={<Plus className="w-3.5 h-3.5 fill-current" />}
                      onClick={() => handleAddGame(item)}
                    />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
