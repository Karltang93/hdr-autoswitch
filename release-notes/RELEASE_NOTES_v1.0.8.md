# HDR Auto-Switch v1.0.8 — Complete Light Mode Overhaul & Clean Documentation Architecture

> **Aesthetics, readability and documentation release: introduces a comprehensive redesign of the Light Mode across all 5 tabs and dialogs, moves all release notes into a clean dedicated archive folder, and reorganizes project documentation with structured, collapsible architecture notes.**

---

### 🎨 What's New & Improved in v1.0.8

#### 1. ☀️ Comprehensive Light Mode Redesign
* **Clean Modern Aesthetic**:
  - The previously harsh dark panels (`#0f0b0b`, `#120d0e`) remaining in Light Mode have been replaced with crisp white cards (`bg-white border-slate-200 shadow-xs`), subtle borders, and balanced typography.
  - **Zero impact on Dark Mode**: The cyberpunk dark theme remains 100% intact with its neon glows and CRT scanline aesthetics.
* **Refined Dashboard (Overview)**:
  - The hero panel now renders an elegant light gradient: an airy sky-blue gradient (`from-sky-50/60 via-white to-white`) with cyan accents in SDR mode, and a warm coral gradient (`from-rose-50 via-white to-white`) with rich coral accents in HDR mode.
  - Aperture dials, monitor cards, recent game posters, and CRT activity logs are fully optimized for high contrast on light backgrounds.
* **Polished My Games & Modals**:
  - Search inputs, grid/list view toggles, launcher category tabs (Steam, Xbox, Other), and game poster cards feature high-contrast text and crisp border styling.
  - Both the **Scan PC for Games** modal and **Manual Add Game** dialog have been upgraded with light modal bodies, high-contrast buttons, and clean form inputs.
* **Vibrant Catalog Browser & Running Windows**:
  - Support tier badges (Native HDR, AutoHDR, Mod/Fix, Limited, Media) received tailored light pastel backgrounds with contrasting typography.
  - Running process cards feature light window icons, clear PID badges, and distinct executable labels.
* **Clean Settings Groups**:
  - Language selection, display targets, switching policies (Exit Only vs. Alt+Tab Debounce), autostart toggles, and blacklist managers are grouped into clean, organized white panels with prominent coral toggles.

#### 2. 📂 Dedicated Release Notes Archive
* All existing release notes (`v1.0.1` through `v1.0.7`) and future release documentation have been organized into a dedicated `release-notes/` folder at the repository root.
* Preserves git revision history and keeps the root workspace directory tidy.

#### 3. 📖 Streamlined & Collapsible Documentation
* The `README.md` file has been cleaned up and streamlined:
  - Replaced the large, dense wall of technical migration and recovery text with structured, collapsible GitHub `<details>` sections and clear bullet points.
  - Retained all authoritative invariant descriptions (provider-specific bindings, `MicrosoftGame.config` parser safety, crash utility quarantine, and atomic settings transactions) in an easily readable format.

#### 4. 🧪 Full Test Suite Pass
* **91 of 91 Frontend unit tests** passing (`npm test`).
* **311 of 311 Rust backend tests** passing (`cargo test`).
* **TypeScript & Vite build** clean compilation (`npm run build`).

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.8_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.8/HDR.Auto-Switch_1.0.8_x64-setup.exe)** | ~3.5 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.8_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.8/HDR.Auto-Switch_1.0.8_x64_en-US.msi)** | ~14.3 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
