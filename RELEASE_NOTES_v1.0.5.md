# HDR Auto-Switch v1.0.5 — The Native HDR Precision, Steam AppID & Multi-Source Release

> **Comprehensive resolution of native HDR detection issues reported by the community: direct Steam AppID pairing (Strategy 0) across 330+ titles, real production executable corrections with `alternate_exes` preservation, bilingual Capcom title parsing, automatic remake year stripping, and multi-source database expansion merging Steam Curator "HDR Games", HDR Gamer, and PCGamingWiki into 1,016 deduplicated verified PC games.**


---

### 🚀 What's New & Fixed in v1.0.5

#### 1. 🎯 Direct Steam AppID Pairing (Strategy 0)
* **Bulletproof Steam Identification**:
  - The PC game scanner parses `appmanifest_{appid}.acf` for all installed games across all Steam libraries.
  - The catalog is now pre-linked with **330+ official Steam AppIDs** (sourced from Steam Curator *HDR Games*).
  - Games from Steam (including *Silent Hill 2, Resident Evil 7: Biohazard, Borderlands GOTY Enhanced, Baldur's Gate 3, Cyberpunk 2077, The Witcher 3, Black Myth: Wukong, Forza Horizon 5, Dead Space, God of War*, etc.) are now recognized **instantly and with 100% precision**.
  - Identification is completely decoupled from heuristic directory naming or guessing executable names — the scanner matches the AppID and directly inspects the hard drive for the true active binary.

#### 2. 🛠️ Real Executables & Alternate Binary Preservation (`alternate_exes`)
* **Real Production Executables**:
  - The original PCGamingWiki Cargo API only exports page names without binary metadata, leading previous versions to synthesize fallback names (e.g. `alanwakeremastered.exe` or `borderlandsgameoftheyearenhanced.exe`).
  - Catalog entries have been updated with verified production executables and alternate binaries:
    - **Alan Wake Remastered**: `game_f_x64_eos.exe` (Epic Games release) + `alanwakeremastered.exe`
    - **Silent Hill 2**: `shproto-win64-shipping.exe` (Steam AppID `2124490`) + `shproto.exe`, `silenthill2.exe`
    - **Borderlands: Game of the Year Enhanced**: `borderlandsgoty.exe` (Steam AppID `729040`, Native HDR) + `borderlandsgameoftheyearenhanced.exe`
    - **Resident Evil 7: Biohazard**: `re7.exe` (Steam AppID `418370`, Native HDR) + `re7trial.exe`, `residentevil7biohazard.exe`
    - **Resident Evil 2, 3, 4, Village**: `re2.exe`, `re3.exe`, `re4.exe`, `re8.exe`
    - **Baldur's Gate 3**: `bg3.exe` (Steam AppID `1086940`) + `bg3_dx11.exe`
    - And dozens more (*Tekken 8, Diablo IV, Street Fighter 6, Dead Island 2, Helldivers 2, Spider-Man 2, S.T.A.L.K.E.R. 2, Space Marine 2...*).
* **Catalog Deduplication Fix**:
  - Resolved a memory bug in `database.rs` where duplicate entries for the same title overwritten previous binaries in memory upon startup.
  - All executable variations are now safely retained in `alternate_exes` and searched across automated scanning, native file browsing, and drag-and-drop.

#### 3. 🌐 Multi-Source Database Expansion & PC-Only Validation (Zero Duplicates)
* **Merged 3 Independent Verified HDR Sources**:
  - **Steam Curator "HDR Games"**: 323 curated Steam games with verified AppIDs, HDR grades, and clear tags separating Native HDR from community mods (RenoDX / Special K).
  - **PCGamingWiki**: Fully preserved, refreshed, and unescaped (fixing raw HTML entities like `&#39;` in titles like *Baldur's Gate 3* or *Tony Hawk's Pro Skater*).
  - **HDR Gamer**: Integrated calibration guides and settings links for verified PC titles.
* **Strict PC-Only Validation (No Console Pollution)**:
  - HDR Gamer covers both console and PC titles. To maintain absolute fidelity for a Windows PC utility, we strictly filter out console-exclusive games (*Demon's Souls, Gran Turismo 7, Astro's Playroom*) as well as games that only feature native HDR on consoles while the PC version is SDR (such as **World of Tanks**, which is now correctly kept in the bottom SDR section!).
* **1,016 Deduplicated Unique PC Games**:
  - Rigorous canonical normalization ensures zero duplicate entries, growing the clean PC catalog from 959 to **1,016 verified PC titles** with 329 pre-linked Steam AppIDs.
  - Lowercase `hdr_type` serialization with enum aliases ensures full cross-version configuration backwards compatibility.


#### 4. 🈳 Bilingual Capcom Titles & Remake Year Stripping
* **Bilingual Slashed Titles**:
  - Steam manifests for Japanese publishers often contain dual language titles (e.g. `RESIDENT EVIL 7 biohazard / BIOHAZARD 7 resident evil`). The scanner now splits on `" / "` and matches against the primary localized name.
* **Remake Year Stripping**:
  - Automatically removes 4-digit years in parentheses (e.g. `Silent Hill 2 (2024)` vs Steam's `SILENT HILL 2`, or `Alone in the Dark (2024)`), preventing normalization mismatches.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.5_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.5/HDR.Auto-Switch_1.0.5_x64-setup.exe)** | ~3.4 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.5_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.5/HDR.Auto-Switch_1.0.5_x64_en-US.msi)** | ~3.4 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
