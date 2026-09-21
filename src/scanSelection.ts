import type { HdrApp } from './types.ts';
import type { ConfigClient, MutationOrigin } from './configState.ts';

export type ScanSelection = Record<string, boolean>;

export function detectionKey(game: HdrApp): string {
  const path = game.path?.replace(/\//g, '\\').toLowerCase()
    .replace(/^\\\\\?\\unc\\/, '\\\\').replace(/^\\\\\?\\/, '') ?? null;
  return JSON.stringify([
    game.launcher?.toLowerCase() ?? null, path, game.exe_name.toLowerCase(), game.steam_id ?? null,
  ]);
}

export function selectDetections(
  games: HdrApp[], selected: (game: HdrApp) => boolean,
): ScanSelection {
  return Object.fromEntries(games.map((game) => [detectionKey(game), selected(game)]));
}

export function toggleDetection(selection: ScanSelection, game: HdrApp): ScanSelection {
  const key = detectionKey(game);
  return { ...selection, [key]: !selection[key] };
}

export function selectedDetections(games: HdrApp[], selection: ScanSelection): HdrApp[] {
  return games.filter((game) => selection[detectionKey(game)])
    .map((game) => ({ ...game, enabled: true }));
}

export async function importSelectedDetections(
  client: Pick<ConfigClient, 'mutate'>, games: HdrApp[], selection: ScanSelection,
  origin: MutationOrigin,
): Promise<number> {
  const detected = selectedDetections(games, selection);
  if (detected.length > 0) {
    await client.mutate('import_detected_games', { detected }, origin);
  }
  return detected.length;
}
