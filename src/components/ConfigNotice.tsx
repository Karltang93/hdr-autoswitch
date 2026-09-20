import { useState } from 'react';
import { configClient, useConfig } from '../useConfig';
import type { HistoryCommand } from '../configState';
import { useI18n } from '../i18n';

export function ConfigNotice({ onSettings }: { onSettings: () => void }) {
  const { snapshot, error, pending } = useConfig();
  const { t } = useI18n();
  const [candidate, setCandidate] = useState('');
  const [resetConfirmed, setResetConfirmed] = useState(false);
  const mode = snapshot?.mode;
  const buttonClass = 'px-3 py-2 border border-[#5accf5]/50 text-[#5accf5] disabled:opacity-40';
  const transition = async (command: HistoryCommand, args?: Record<string, unknown>) => {
    try {
      await configClient.changeHistory(command, args);
      setResetConfirmed(false);
      setCandidate('');
    } catch (failure) {
      configClient.reportError(failure);
    }
  };
  const needsTarget = snapshot?.mode === 'ready' && snapshot.settings.target_monitor.kind === 'needs_confirmation';
  const needsConsent = snapshot?.mode === 'ready' && snapshot.settings.switch_method === 'shortcut';

  if (snapshot?.mode === 'ready' && !error && !snapshot.controller_issue && !needsTarget && !needsConsent && !pending) {
    return null;
  }

  return (
    <section className="mb-5 p-4 border border-amber-400/50 bg-[#180e10] text-xs text-amber-200 space-y-3" aria-live="polite">
      {!snapshot && <p>{error ? t.configUnavailable : t.configLoading}</p>}
      {error && <p role="alert">{t.configError} {error}</p>}
      {snapshot?.issue && <p role="alert">{snapshot.issue}</p>}
      {snapshot?.controller_issue && <p role="alert">{t.configControlPaused}: {snapshot.controller_issue}</p>}
      {snapshot?.mode === 'ready' && snapshot.controller_issue && <button className={buttonClass} disabled={pending}
        onClick={() => { void configClient.mutate('recheck_controller').catch(configClient.reportError); }}>
        {t.configRecheck}
      </button>}
      {pending && <p role="status">{t.configSaving}</p>}
      {mode === 'first_run' && <>
        <p>{t.configFirstRun}</p>
        <button className={buttonClass} disabled={pending} onClick={() => transition('initialize_config')}>
          {t.configInitialize}
        </button>
      </>}
      {mode === 'import_available' && <>
        <p>{t.configImportAvailable}</p>
        <button className={buttonClass} disabled={pending} onClick={() => transition('import_legacy_config')}>
          {t.configImport}
        </button>
      </>}
      {snapshot?.mode === 'recovery_required' && <>
        <p>{t.configRecovery}</p>
        <label className="block">
          {t.configSelectRecovery}
          <select
            className="block mt-2 p-2 bg-[#0f0b0b] border border-amber-400/50 w-full"
            value={candidate}
            onChange={(event) => setCandidate(event.target.value)}
            disabled={pending}
          >
            <option value="">{t.configSelectRecovery}</option>
            {snapshot.candidates.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}
          </select>
        </label>
        <button className={buttonClass} disabled={pending || !candidate} onClick={() => transition('restore_config', { candidateId: candidate })}>
          {t.configRestore}
        </button>
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={resetConfirmed} disabled={pending} onChange={(event) => setResetConfirmed(event.target.checked)} />
          {t.configResetConfirm}
        </label>
        <button className={buttonClass} disabled={pending || !resetConfirmed} onClick={() => transition('reset_config')}>
          {t.configReset}
        </button>
      </>}
      {mode === 'unsupported_schema' && <p>{t.configUnsupported}</p>}
      {mode === 'unavailable' && <p>{t.configUnavailable}</p>}
      {(mode === 'unavailable' || mode === 'recovery_required') && <p>{t.configRestartHint}</p>}
      {needsTarget && <p>{t.configConfirmTarget}</p>}
      {needsConsent && <p>{t.configNativeConsent}</p>}
      {(needsTarget || needsConsent) && <button className={buttonClass} onClick={onSettings}>{t.navSettings}</button>}
      {snapshot && mode !== 'ready' && <p className="break-all">{t.configFile}: {snapshot.config_path}</p>}
    </section>
  );
}
