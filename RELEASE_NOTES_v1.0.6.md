# HDR Auto-Switch v1.0.6 — Persistent Monitor Identity & Hardened HDR Control

> **Major stability and architecture release: fixes the bug where target display selection reverted to "All Monitors" after PC restart via durable Windows device-interface paths, introduces transactional schema-2 settings storage with automatic recovery, adds a 1 Hz foreground watchdog fallback, silent minimized startup, complete English/Czech localization, and full integration of the 1,027 verified PC HDR game catalog.**

---

### 🚀 What's New & Fixed in v1.0.6

#### 1. 🖥️ Persistent Monitor Identity Across PC Restarts
* **The Problem**:
  - Previously, saved display choices stored Windows runtime adapter and target IDs (`LUID` and `target_id`).
  - Windows dynamically reallocates these numeric identifiers upon system reboots, monitor sleep/wake cycles, or graphics driver updates.
  - On the next startup, the app failed to match the saved runtime ID, causing it to fall back to `"All Monitors"` and overwrite the configuration file.
* **The Solution**:
  - Target display selection now persists the **opaque Windows device-interface path** (`\\?\DISPLAY#...`), which remains strictly immutable across reboots, GPU driver reloads, and cable reconnects.
  - Primary display resolution now references real Windows source-primary metadata instead of volatile target indices.
  - If a display is slow to wake or in deep standby during boot, the application enters a protected retry state rather than erasing the user's setting.

#### 2. 🛡️ Transactional Schema-2 Settings Storage & Recovery
* **Atomic Staged Writes & Journal Checkpoints**:
  - Naive file overwrites have been replaced by strict schema-2 transactional storage (`config_storage.rs`).
  - Writes are staged and validated before committing, with journal checkpoints and preserved recovery evidence.
* **Read-Only Fencing on Load Failures**:
  - If settings fail to load or parse, the application fences write access and preserves recovery evidence.
  - A later save or background event will **never** silently overwrite the user's custom library or preferences with empty defaults.

#### 3. ⏱️ 1 Hz Foreground Watchdog Fallback
* **Never Miss a Game Launch**:
  - While zero-overhead OS hooks (`SetWinEventHook` with `EVENT_SYSTEM_FOREGROUND`) remain the primary detection mechanism, Windows can occasionally drop or defer events during heavy GPU load or UAC elevation.
  - A lightweight 1 Hz watchdog verifies the foreground process identifier and triggers immediate state recovery if an OS event was dropped.

#### 4. 🤫 Silent Minimized Startup & Duplicate Launch Protection
* **Silent System Boot**:
  - When configured to launch on Windows startup or with the `--minimized` flag, the application starts hidden in the system tray without popping up or flashing an empty window on the desktop.
* **Single Instance Protection**:
  - Launching a second instance smoothly brings the existing window to the foreground instead of crashing or generating duplicate tray icons.

#### 5. 🌐 Full English & Czech UI and Catalog Localization
* Complete, high-fidelity English and Czech translation across:
  - Settings, dashboard, application manager, and manual add modals.
  - Tray context menu, badges, notifications, and telemetry text.
  - Catalog descriptions, notes, and calibration guide links.

#### 6. 🎮 Verified 1,027 PC HDR Games & 347 Steam AppIDs (Zero Duplicates)
* Fully incorporates the verified multi-source catalog:
  - **Steam Curator "HDR Games"**: 347 directly linked official Steam AppIDs for instant Strategy-0 detection.
  - **PCGamingWiki**: Cleaned production executables (*Alan Wake Remastered*, *Silent Hill 2*, *Resident Evil 7*, *Borderlands GOTY Enhanced*, *Baldur's Gate 3*, *Tony Hawk's Pro Skater 1 + 2*, etc.).
  - **HDR Gamer**: Integrated calibration profiles for verified PC titles (*Ghostrunner 1 & 2*, *Mass Effect Legendary Edition*, *Gears of War 4*, *Grounded*, *Farming Simulator 22*, *Mafia III: DE*, *Psychonauts 2*, *The Quarry*, etc.).
  - **Zero Duplicates**: Complete audit eliminating trademark (`™`, `®`) and Roman numeral collisions.
  - **Console Exclusions**: Explicitly excludes console-only HDR titles (*World of Tanks*, *Ghost Recon: Wildlands*, *Homefront: The Revolution*, *State of Decay 2*), ensuring they remain in the bottom SDR section as intended.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.6_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.6/HDR.Auto-Switch_1.0.6_x64-setup.exe)** | ~3.4 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.6_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.6/HDR.Auto-Switch_1.0.6_x64_en-US.msi)** | ~3.4 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
