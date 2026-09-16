# HDR Auto-Switch v1.0.2 — Windows 11 24H2 & Smart Alt+Tab Update

> **Full Windows 11 24H2 & OLED display compatibility, instant game exit detection, seamless Alt+Tab behavior without monitor blinking, and false-positive filter overhaul.**

---

### 🚀 What's New & Fixed in v1.0.2

#### 1. 🖥️ Windows 11 24H2 & OLED / WCG Display Compatibility (Issue Fix)
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

#### 3. 🛡️ System App Catalog False-Positive Elimination
* Fixed a substring matching bug in the catalog lookup where common executables like `snippingtool.exe`, `explorer.exe`, `chrome.exe`, and `applicationframehost.exe` were mistakenly enrolled as "NATIVE HDR" games.
* Strengthened the game matching engine with strict boundary checks.
* Automatically purges legacy false-positive entries from `config.json` and recent games telemetry upon startup.

#### 4. ⚙️ Settings & Telemetry UI Updates
* Added an interactive **HDR Switching Policy** selector under Settings.
* Recent Games history now filters out background system utilities and displays live hook event status.
* Full bilingual support (English & Czech) for all new policy options.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR.Auto-Switch_1.0.2_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.2/HDR.Auto-Switch_1.0.2_x64-setup.exe)** | ~3.3 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR.Auto-Switch_1.0.2_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.2/HDR.Auto-Switch_1.0.2_x64_en-US.msi)** | ~3.3 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
