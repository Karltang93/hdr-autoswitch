# HDR Auto-Switch v1.0.4 — The Discovery, Migration & Precision Release

> **Comprehensive multi-drive PC game scanner overhaul (Steam .vdf/.acf across all drives, Epic Games, GOG, Windows Registry), smart library state badges with drive migration detection, strict title matching (eliminating prequel/sequel false matches like Assetto Corsa), native Windows file picker ("Browse..."), instant Drag & Drop .exe support, and 1280×720 window expansion.**

---

### 🚀 What's New & Fixed in v1.0.4

#### 1. 🔍 Multi-Drive PC Game Scanner Overhaul
* **Deep Multi-Drive Scanner Engine**:
  - **Steam Integration**: Discovers all library folders across all storage drives (`C:`, `D:`, `E:`, etc.) via `libraryfolders.vdf` and parses individual `appmanifest_*.acf` files. High-speed traversal automatically resolves modern shipping binaries (e.g. `Binaries/Win64/*-Shipping.exe`) in under 2.5 seconds while skipping assets/content/textures folders.
  - **Epic Games Store**: Scans launcher manifests (`%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests/*.item`) to locate actual install roots and launch executables.
  - **GOG Galaxy & Windows Registry**: Identifies standalone game installs with safe path validation, filtering out desktop utilities, runtimes, and background launchers.
* **Clear Categorization & Visual Badges**:
  - **Top Section (HDR & Auto HDR)**: Games with verified HDR support are grouped at the top with glowing badges and pre-checked (`[x]`) by default.
  - **Bottom Section (SDR Games)**: Other installed games are grouped below as `OTHER INSTALLED GAMES — SDR (COMPATIBLE WITH RTX HDR / MODS)`, un-checked (`[ ]`) by default so you can easily enable HDR tracking for RTX HDR or community mods.
  - **Library State Badges**:
    - `★ NEW`: Newly discovered HDR games not yet in your library (pre-selected).
    - `✓ IN LIBRARY`: Games already tracked with up-to-date paths (un-checked by default).
    - `⚡ UPDATE PATH`: Detects when a previously tracked game has moved to another drive or folder (pre-selected, displaying `➔ New location: D:\...`).
  - **Quick Action Selectors**: `[ Select All HDR ]`, `[ Select All ]`, and `[ Deselect All ]` buttons.

#### 2. 🎯 Strict Title Matching & Franchise Disambiguation
* **Prequel vs. Sequel Disambiguation**:
  - Fixed a loose substring matching bug where base games falsely matched spin-offs or sequels (*Assetto Corsa* falsely matching *Assetto Corsa Competizione*, *Doom* matching *Doom Eternal*, or *Spider-Man* matching *Spider-Man 2*).
  - Added edition stripping (*Director's Cut, Remastered, Game of the Year, Enhanced Edition*) and safe asymmetrical subtitle parsing (games in the same franchise like *Star Wars: Squadrons* vs *Star Wars: Outlaws* will never falsely cross-match).
* **Catalog Additions**:
  - Added standard **Assetto Corsa** (`acs.exe`, `assettocorsa.exe`, `acs_x86.exe`, `Custom` / `manual_fix` for CSP + Pure mod).
  - Updated **Assetto Corsa Competizione** with its actual Unreal Engine 4 binaries (`ac2-win64-shipping.exe`, `acc.exe`).
  - Added native HDR profiles for **Silent Hill 2 Remake** (UE5) and **Dead Island 2**, plus strengthened detection for **Alan Wake 2**.

#### 3. 📂 Native File Picker ("Browse / Procházet...")
* Added a native 64-bit Windows file selection dialog to the "Add Game Manually" modal.
* Click **"Browse... / Procházet..."** or the folder icon to select any game executable directly from your storage.
* Automatically inspects the chosen executable:
  - Extracts executable name and full directory path.
  - Matches against the HDR database to auto-fill the game title and recommend the optimal HDR tier.

#### 4. 🖱️ Drag-and-Drop .exe Support
* **Global Drag & Drop**: Drag any `.exe` from Windows Explorer directly into the application window. HDR Auto-Switch will inspect the binary, detect its HDR support, and open the confirmation dialog immediately.
* **In-Modal Dropzone**: An interactive, animated dashed dropzone within the Manual Add modal.

#### 5. 💽 Drive Relocation & Library Path Verification
* Added real-time disk path verification: If a previously added game executable was moved or uninstalled, the library cards (Grid and List views) display a `[FILE NOT FOUND]` indicator with a prompt to run the PC scan to update its location.
* Importing a moved game through the scanner automatically updates its `path`, `launcher`, and `steam_id`.

#### 6. 📐 Window Ergonomics
* Increased default window dimensions to **1280 × 720 px** for improved layout spacing and readability across multi-column library views.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.4_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.4/HDR.Auto-Switch_1.0.4_x64-setup.exe)** | ~3.4 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.4_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.4/HDR.Auto-Switch_1.0.4_x64_en-US.msi)** | ~3.4 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
