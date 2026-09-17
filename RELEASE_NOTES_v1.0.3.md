# HDR Auto-Switch v1.0.3 — The Clean Release

> **Full Windows 11 24H2 & OLED display compatibility, instant game exit detection, seamless Alt+Tab behavior without monitor blinking, purged mockup telemetry, and pixel-perfect logo alignment.**

---

### 🚀 What's New & Fixed in v1.0.3

#### 1. 🖥️ Windows 11 24H2 & OLED / WCG Display Compatibility
* **Resolved false HDR detection on OLEDs (e.g., Samsung Odyssey G73SH)**: Windows 11 24H2 introduced Auto Color Management (ACM) and Wide Color Gamut (WCG) which activated legacy `advancedColorEnabled` flags in SDR mode. The app previously thought HDR was already enabled and skipped turning it on.
* **Modern Windows Display APIs**:
  - Implemented `DisplayConfigGetAdvancedColorInfo2` (Type 15) to inspect actual active color mode and verify `highDynamicRangeUserEnabled` directly.
  - Implemented `DisplayConfigSetHdrState` (Type 16), the official Windows 11 24H2 method for switching HDR.
  - Retained robust fallback chain: Type 16 -> Type 10 -> Hardware-level virtual `Win+Alt+B` hotkey injection.

#### 2. ⚡ Seamless Alt+Tab & Instant Game Exit
* **No more monitor flickering or game swapchain desync**: Switching between an HDR game and Discord/browser previously triggered a display renegotiation (blackout) and caused games to get stuck in mixed SDR/HDR states.
* **New Switching Policy (Enabled by Default)**:
  - **Only on Game Exit (Recommended)**: Keeps HDR active while the game process is running, even when you Alt+Tab to desktop apps.
  - **On Window Switch (Alt+Tab)**: Reverts to SDR whenever you leave the game window, after a customizable delay (0–10s).
* **Instant SDR Return on Game Close**: Integrated kernel-level process tracking (`WaitForSingleObject`). When your game closes, SDR is restored instantly with 0ms delay.

#### 3. 🎯 Pixel-Perfect Logo Centering & Aspect Ratio
* Fixed vertical stretching and uncentered padding on the diamond aperture logos in both the top header and hero banner.
* Containers now maintain an exact 1:1 aspect ratio with balanced padding on all sides.

#### 4. 🧹 Clean Telemetry Feed (No More Mockup Games)
* Removed initial hardcoded mockup/prototype games (Bodycam, Assetto Corsa, etc.) from the telemetry feed.
* Automatically purges legacy placeholder cards from local storage on launch.
* Added an elegant empty-state indicator until your first real HDR game is played.

#### 5. 🗂️ Intelligent Game Deduplication & Stem Matching
* Resolved duplicate cards caused by shipping or launcher binaries (e.g., `bodycam.exe` vs `Bodycam-Win64-Shipping.exe`).
* Automatically correlates shipping binaries with parent titles, preserves and merges Steam IDs, artwork covers, and tags.

#### 6. 🛡️ System App Catalog False-Positive Elimination
* Fixed a substring matching bug in catalog lookup where common utilities like `snippingtool.exe`, `explorer.exe`, `chrome.exe`, and `applicationframehost.exe` were mistakenly enrolled as games.
* Purged system applications from active libraries and strengthened matching rules.

#### 7. 📐 Wider Default Window Geometry (1120 × 720)
* Widened the default window size to 1120px to prevent header controls from clipping and allow all 6 Recent Games cards to display neatly side-by-side.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.3_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.3/HDR.Auto-Switch_1.0.3_x64-setup.exe)** | ~3.3 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.3_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.3/HDR.Auto-Switch_1.0.3_x64_en-US.msi)** | ~3.3 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
