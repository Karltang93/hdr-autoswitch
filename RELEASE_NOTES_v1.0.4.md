# HDR Auto-Switch v1.0.4 — The Discovery & Precision Release

> **Full PC Game Scanner overhaul with multi-drive Steam, Epic Games & GOG support, grouped HDR (ON) vs SDR (OFF) scan results, native Windows executable file picker ("Browse..."), instant Drag & Drop .exe support, and Silent Hill 2 / Alan Wake 2 catalog integration.**

---

### 🚀 What's New & Fixed in v1.0.4

#### 1. 🔍 Comprehensive Game Scanner Overhaul
* **Detects All Installed Games Across All Drives**:
  - **Steam**: Seamless multi-drive parsing via `libraryfolders.vdf` and `appmanifest_*.acf` across all drives (`C:`, `D:`, `E:`, etc.), resolving deep Unreal Engine / Northlight binaries (`Binaries/Win64/*-Shipping.exe`).
  - **Epic Games Store**: Parses launcher item manifests (`%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests`), accurately matching titles and installation folders.
  - **GOG Galaxy & Windows Registry**: Detects standalone installs and GOG games with safe path inspection, skipping non-game Windows software, runtimes, and launcher shells.
* **Grouped Scan Results (HDR vs SDR)**:
  - **Top Section (HDR & Auto HDR)**: Games with verified HDR support in the database are grouped at the top with glowing badges and pre-selected (`[x]` ON) by default.
  - **Bottom Section (SDR Games)**: All other installed games are listed in a separate section below, un-checked (`[ ]` OFF) by default, allowing users to enable Auto HDR or RTX HDR for any game with a single click.
  - **Quick Select Controls**: Added `[ Select All HDR ]`, `[ Select All ]`, and `[ Deselect All ]` buttons.

#### 2. 📂 Native File Picker ("Browse / Procházet...")
* Added a native 64-bit Windows file selection dialog to the "Add Game Manually" modal.
* Click **"Browse... / Procházet..."** or the folder icon next to the executable field to select any game `.exe` directly from your disk.
* Automatically inspects the chosen executable:
  - Extracts the clean `.exe` name and full directory path.
  - Matches the game against the HDR database to auto-fill the game title and recommend the optimal HDR tier (Native HDR vs Auto HDR).

#### 3. 🎯 Drag-and-Drop .exe Support
* Added interactive Drag-and-Drop dropzone to the Manual Add modal.
* **Global Drag & Drop**: You can now drag any game `.exe` directly from Windows Explorer into the app window. HDR Auto-Switch will inspect the binary, auto-populate the details and HDR status, and present the add confirmation instantly.

#### 4. 🎮 Expanded HDR Database
* Added native HDR support for:
  - **Silent Hill 2 Remake** (UE5: `SHProto-Win64-Shipping.exe`, `SHProto.exe`, `silenthill2.exe`).
  - **Dead Island 2** (`DeadIsland-Win64-Shipping.exe`, `DeadIsland.exe`, `deadisland2.exe`).
  - Strengthened matching for **Alan Wake 2** on Epic Games and other modern shipping binaries.

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
