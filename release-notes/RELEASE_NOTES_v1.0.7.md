# HDR Auto-Switch v1.0.7 — Provider Executable Authority, Helper Quarantine & Bounded Retry Architecture

> **Hardened stability and architecture release: introduces strict storefront provider executable authority (Steam, Xbox, Epic, GOG, Windows), declarative Xbox `MicrosoftGame.config` parsing, runtime quarantine for game helpers and crash reporters, multi-installation disambiguation, and bounded foreground observation retries for anti-cheat protected titles. Special thanks to [@Karltang93](https://github.com/Karltang93) for contributing [PR #3](https://github.com/Soptik1290/hdr-autoswitch/pull/3)!**

---

### 🚀 What's New & Improved in v1.0.7

#### 1. 🛡️ Storefront Provider Executable Authority
* **Fail-Closed Provider Boundaries**:
  - Automatic game discovery now enforces strict, provider-scoped executable authority across **Steam**, **Xbox / Microsoft Store**, **Epic Games**, **GOG**, and **Windows**.
  - Loose basename matching, broad subfolder crawling, and title stem heuristics have been eliminated from automatic discovery paths.
  - Games must be positively authorized by their respective store provider or explicit user action before being nominated.

#### 2. 🎮 Declarative Xbox / Microsoft Store Config Resolution
* **Safe, Hardened `MicrosoftGame.config` Parser**:
  - Automatically discovers primary game executables declared in `MicrosoftGame.config` (e.g. *Age of Empires III: Definitive Edition* and Xbox Game Pass titles) without guessing.
  - Implements a hardened, bounded XML parser with strict limits (64 KiB file size, 1,024 nodes, 32 nesting levels, local non-network roots only).
  - Fully immune to XXE (XML External Entity) attacks, XML bombs, and symlink/junction path traversal.

#### 3. 🚫 Runtime Quarantine for Helper Executables & 1-Click Repair
* **Automated Quarantine Protection**:
  - Prevents HDR toggling on game launchers, crash reporters, and updater utilities (`GameLaunchHelper.exe`, `BsSndRpt.exe`, `BsSndRpt64.exe`, `BugSplat.exe`, `BugSplatHD64.exe`).
  - If a user library previously nominated a helper executable, it is automatically derived into quarantine at runtime without corrupting user settings.
* **1-Click Targeted Executable Repair**:
  - Quarantined apps trigger a clear warning badge in the UI and system tray.
  - A dedicated **Repair** action lets users pick the actual game executable directly, cleanly discarding historical helper aliases while keeping all custom HDR settings intact.

#### 4. ⚡ Bounded Foreground Retry Architecture (Zero-Overhead & Anti-Cheat Resilience)
* **Smooth Handling of Anti-Cheat Protected Games**:
  - Games protected by anti-cheat engines (Easy Anti-Cheat, BattlEye) or elevated Windows permissions can temporarily deny or delay `OpenProcess` / `QueryFullProcessImageNameW` inspection.
  - Instead of re-querying and consuming CPU cycles on focus shifts, the monitor hook applies **bounded exponential backoff** (delays: 1s, 2s, 4s, followed by a slow 30s recovery probe).
  - Once successfully observed, the process identity and creation timestamp are cached, maintaining true **0.0% background CPU usage**.

#### 5. 🔄 Multi-Installation Disambiguation & Conflicting Batch Rejection
* **Multi-Storefront Support**:
  - Detects and tracks separate installations of the same game across different storefronts (e.g. a Steam installation alongside a GOG or Xbox Game Pass installation) with independent identities.
* **Atomic Import Fences**:
  - If an import batch contains conflicting paths or duplicate primary bindings, the batch is safely rejected before mutating configuration.

#### 6. 🎯 Exact Running Processes Path Authorization
* **Unified Authority Between UI and Backend**:
  - The *Running Processes* tab now queries the backend's authoritative runtime path resolver, eliminating any discrepancy between UI display badges and native HDR switching logic.

---

### 💖 Special Thanks & Contributors
* Huge thanks to **[@Karltang93](https://github.com/Karltang93)** for contributing [Pull Request #3](https://github.com/Soptik1290/hdr-autoswitch/pull/3), implementing provider executable authority, declarative Xbox configuration resolution, runtime helper quarantine, and the bounded foreground retry architecture!

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.7_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.7/HDR.Auto-Switch_1.0.7_x64-setup.exe)** | ~3.5 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.7_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.7/HDR.Auto-Switch_1.0.7_x64_en-US.msi)** | ~14.3 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (21H2, 22H2, 23H2, 24H2)
* **Display**: At least one HDR10 / VESA DisplayHDR / OLED capable monitor
