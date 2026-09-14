# 🎮 HDR Auto-Switch pro Windows

Moderní, blesková a extrémně úsporná aplikace pro automatické zapínání a vypínání HDR ve Windows 10 a 11.

Navržena tak, aby měla **nulovou zátěž na procesor (0.0 % CPU v klidu)** a zabírala jen minimum operační paměti (~15–25 MB RAM). Už žádné manuální mačkání zkratky `Win + Alt + B` před každým hraním a po něm.

---

## ✨ Klíčové funkce

* 🚀 **0.0% CPU zátěž (Žádný polling):** Místo náročných cyklů na pozadí (`setInterval` / `while True`) využívá nativní Windows OS Event Hook (`SetWinEventHook` s `EVENT_SYSTEM_FOREGROUND`). Procesor je 99.99 % času zcela v klidu.
* 🖥️ **Nativní Win32 DisplayConfig API:** Přepíná HDR přímo na úrovni ovladače displeje, bez závislosti na Xbox Game Baru a bez simulace kláves.
* 🎯 **Podpora per-monitor nastavení:** Schopnost přepínat buď všechny HDR monitory, nebo pouze vámi vybraný konkrétní displej (detekuje názvy monitorů, např. *LG UltraGear*, *Samsung Odyssey*).
* ⏱️ **Alt+Tab ochrana (Debounce):** Když odskočíte ze hry na 2 sekundy do Discordu nebo webu, monitor okamžitě neproblikne. Nastavitelná prodleva (0s, 1s, 2s, 3s, 5s).
* 📚 **Rozsáhlá databáze her:**
  * **Nativní HDR tituly:** *Cyberpunk 2077, Elden Ring, Alan Wake 2, Baldur's Gate 3, Black Myth: Wukong, God of War, Horizon Forbidden West, Red Dead Redemption 2, The Witcher 3* a desítky dalších.
  * **Auto HDR hry:** Integrovaný katalog z [PCGamingWiki List of games that support Auto HDR](https://www.pcgamingwiki.com/wiki/List_of_games_that_support_Auto_HDR) (*Skyrim SE, Fallout 4, GTA V, Dark Souls III, BioShock Infinite* atd.).
  * **Přehrávače médií:** *mpv, MPC-HC, VLC, PotPlayer, Kodi*.
  * **Online synchronizace:** 1-click aktualizace katalogu z internetu.
* 🚫 **Chytrý Blacklist prohlížečů:** Webové prohlížeče (*Chrome, Edge, Firefox, Brave*) a aplikace jako *Discord* či *Spotify* jsou ve výchozím stavu vyloučeny, aby běžné surfování po webu zbytečně nepřepínalo displej do HDR.
* ⚡ **1-Click přidání z běžících procesů:** V záložce *Běžící okna* vidíte všechny otevřené aplikace a kliknutím na `+ Přidat do HDR` můžete libovolnou hru okamžitě přidat.
* 🔔 **Windows Toast Notifikace:** Zobrazí systémové oznámení při aktivaci nebo deaktivaci HDR (lze vypnout v nastavení).
* 🪟 **Frosted Glass (Mica/Acrylic) vzhled:** Moderní Windows 11 design s poloprůhlednými skleněnými panely, automatickou detekcí a přepínačem Tmavého / Světlého režimu.
* 📌 **Systémová lišta & Autostart:**
  * Běží v oznamovací oblasti (System Tray).
  * Zavření křížkem aplikaci neukončí, ale minimalizuje do lišty.
  * Možnost automatického spuštění při přihlášení do Windows.

---

## 🛠️ Použité technologie

* **Framework:** [Tauri v2](https://v2.tauri.app/)
* **Backend:** Rust (`windows-rs` Win32 API, `tauri-plugin-autostart`, `tauri-plugin-notification`, `reqwest`)
* **Frontend:** React 19, TypeScript, Vite, Tailwind CSS v4, Lucide Icons

---

## 🚀 Spuštění a vývoj

### Požadavky
* [Node.js](https://nodejs.org/) (v20+)
* [Rust](https://www.rust-lang.org/) (cargo & rustc)
* Windows 10 (build 17763+) nebo Windows 11

### Instalace závislostí
```bash
npm install
```

### Spuštění vývojové verze (Dev)
```bash
npm run tauri dev
```

### Sestavení instalačního balíčku / .exe (Build)
```bash
npm run tauri build
```
Výsledný zkompilovaný instalátor a samostatný `.exe` soubor najdete ve složce:
`src-tauri/target/release/`

---

## 📄 Licence
MIT License
