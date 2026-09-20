import type { HdrApp } from './types.ts';

type LibraryIdentity = Pick<HdrApp, 'exe_name'>
  & Partial<Pick<HdrApp, 'name' | 'steam_id' | 'alternate_exes'>>;

function conflictingIds(existing: HdrApp, item: LibraryIdentity): boolean {
  return existing.steam_id != null && item.steam_id != null && existing.steam_id !== item.steam_id;
}

function executableOverlap(existing: HdrApp, item: LibraryIdentity): boolean {
  const incoming = new Set([item.exe_name, ...(item.alternate_exes ?? [])].map((exe) => exe.toLowerCase()));
  return [existing.exe_name, ...(existing.alternate_exes ?? [])]
    .some((exe) => incoming.has(exe.toLowerCase()));
}

export function findLibraryApp(apps: HdrApp[], item: LibraryIdentity): HdrApp | undefined {
  return apps.find((existing) => {
    if (existing.exe_name.toLowerCase() === item.exe_name.toLowerCase()) return true;
    if (conflictingIds(existing, item)) return false;
    return (item.name != null && existing.name.toLowerCase() === item.name.toLowerCase())
      || executableOverlap(existing, item);
  });
}

export function findTrackedApp(apps: HdrApp[], item: LibraryIdentity): HdrApp | undefined {
  return apps.find((existing) => existing.enabled
    && (existing.exe_name.toLowerCase() === item.exe_name.toLowerCase()
      || (!conflictingIds(existing, item) && executableOverlap(existing, item))));
}
