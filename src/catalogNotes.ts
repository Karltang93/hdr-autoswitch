import type { Language } from './i18n.ts';

const englishNotes: Readonly<Record<string, string>> = {
  'Doporučen mód Special K (HDR retrofit)': 'Special K mod recommended (HDR retrofit)',
  'HDR je ve hře trvale zapnuto pokud je v OS aktivní': 'HDR stays enabled in the game while Windows HDR is active',
  'High-performance video přehrávač s pokročilým HDR tone mappingem': 'High-performance video player with advanced HDR tone mapping',
  'Mediální centrum s podporou HDR': 'Media center with HDR support',
  'Nativní Decima Engine HDR': 'Native Decima Engine HDR',
  'Nativní Game & Cinematic HDR': 'Native Game & Cinematic HDR',
  'Nativní HDR': 'Native HDR',
  'Nativní HDR (Vulkan & DX11)': 'Native HDR (Vulkan & DX11)',
  'Nativní HDR podpora': 'Native HDR support',
  'Nativní HDR podpora (DirectX 11/12)': 'Native HDR support (DirectX 11/12)',
  'Nativní HDR podpora (Hitman 3)': 'Native HDR support (Hitman 3)',
  'Nativní HDR podpora (PCGamingWiki)': 'Native HDR support (PCGamingWiki)',
  'Nativní HDR podpora (Uncharted 4 & The Lost Legacy)': 'Native HDR support (Uncharted 4 & The Lost Legacy)',
  'Nativní HDR podpora v grafickém nastavení (PCGamingWiki)': 'Native HDR support in graphics settings (PCGamingWiki)',
  'Nativní HDR v DX12': 'Native HDR in DX12',
  'Nativní Unreal Engine 5 HDR': 'Native Unreal Engine 5 HDR',
  'Nativní Unreal Engine 5 HDR podpora': 'Native Unreal Engine 5 HDR support',
  'Nativní idTech 7 HDR': 'Native idTech 7 HDR',
  'Nativní podpora': 'Native support',
  'Nativní podpora HDR': 'Native HDR support',
  'Nativní podpora HDR10': 'Native HDR10 support',
  'Next-gen nativní HDR update': 'Next-gen native HDR update',
  'Omezená nativní podpora HDR (PCGamingWiki)': 'Limited native HDR support (PCGamingWiki)',
  'Omezená podpora (vyžaduje fullscreen v nativním rozlišení)': 'Limited support (requires fullscreen at native resolution)',
  'Plná nativní HDR podpora': 'Full native HDR support',
  'Podporuje Microsoft Windows Auto HDR': 'Supports Microsoft Windows Auto HDR',
  'Přehrávač médií s podporou HDR': 'Media player with HDR support',
  'Přehrávač médií s podporou madVR / HDR': 'Media player with madVR / HDR support',
  'Přehrávač s podporou MPC Video Renderer HDR': 'Player with MPC Video Renderer HDR support',
  'Přehrávač s podporou madVR HDR passthrough': 'Player with madVR HDR passthrough support',
  'Přehrávač videa s podporou HDR': 'Video player with HDR support',
  'RE Engine nativní HDR': 'RE Engine native HDR',
  'Snowdrop Engine nativní HDR': 'Snowdrop Engine native HDR',
  'Trvale aktivní v enginu (PCGamingWiki)': 'Always enabled in the engine (PCGamingWiki)',
  'Unreal Engine 5 nativní HDR': 'Unreal Engine 5 native HDR',
  'Vyžaduje Custom Shaders Patch (CSP) + Pure mod pro HDR': 'Requires Custom Shaders Patch (CSP) + Pure mod for HDR',
  'Vyžaduje DX11 režim pro stabilní HDR': 'Requires DX11 mode for stable HDR',
  'Vyžaduje úpravu / mod / Special K (PCGamingWiki)': 'Requires a fix / mod / Special K (PCGamingWiki)',
  'Windows 11 Auto HDR ověřeno': 'Windows 11 Auto HDR verified',
  'Špičková nativní HDR kalibrace': 'High-quality native HDR calibration',
};

export function catalogNotes(notes: string | null | undefined, language: Language): string {
  if (!notes || language === 'cs') return notes ?? '';
  return notes.split(' | ').map((part) => englishNotes[part] ?? part).join(' | ');
}

export function launcherName(launcher: string, language: Language): string {
  if (launcher === 'Média' || launcher === 'Media') return language === 'cs' ? 'Média' : 'Media';
  return launcher;
}
