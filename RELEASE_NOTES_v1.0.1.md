# HDR Auto-Switch v1.0.1 — "Set & Forget" Update

> **True zero-overhead background operation, silent system tray autostart, and automatic HDR game enrollment.**

---

### 🚀 What's New in v1.0.1

#### 1. 🔕 Start Minimized to System Tray
* Added a new configuration option: **Start Minimized to Tray** (`start_minimized`).
* When enabled (or when started via Windows Autostart with `--minimized`), the application launches silently into the notification area (system tray) without popping up on the desktop.
* Click the system tray icon anytime to open and focus the dashboard.

#### 2. 💤 Zero-Overhead GUI Hibernation (0.0% CPU / GPU)
* When the app window is hidden or minimized to the tray, the frontend automatically suspends its GSAP animation loops (`gsap.ticker.sleep()`) and halts rendering.
* Combined with Windows WebView2 process throttling, background resource consumption drops to **0.0% CPU & GPU**, preserving 100% of system performance for your games.
* Resumes instantaneously with full state refresh upon restoring the window.

#### 3. 🎮 Dynamic HDR Game Auto-Enrollment
* **Install and play**: When launching any HDR-supported game from the built-in 949+ curated database (including Unreal Engine `-Win64-Shipping` executables, custom launchers, etc.), the native `SetWinEventHook` detects the game window in real-time.
* If the game hasn't been added to your library yet, it is **automatically enrolled on the fly** and switches your displays to HDR10 immediately!
* Shows a clean native notification: *"HDR enabled for [Game Name]"*.

#### 4. 🔄 Periodic Silent Database Sync & Background Scan
* The application quietly verifies and downloads newly cataloged HDR titles from PCGamingWiki once a week in a lightweight background worker.
* Runs a non-blocking background disk scan after boot to pair existing games with Steam IDs, artwork, and alternate executables without user intervention.

#### 5. 🌐 UI & Settings Enhancements
* Added dedicated toggle switches in **Settings > System Integration** for tray autostart, auto-detection, and auto-sync.
* Full localization support in both **English** and **Czech (Čeština)**.

---

### 📦 Downloads & Installation

| Asset | Size | Description |
| :--- | :--- | :--- |
| 💿 **[HDR Auto-Switch_1.0.1_x64-setup.exe](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.1/HDR.Auto-Switch_1.0.1_x64-setup.exe)** | ~3.3 MB | Recommended NSIS installer with desktop & start menu shortcuts |
| 📦 **[HDR Auto-Switch_1.0.1_x64_en-US.msi](https://github.com/Soptik1290/hdr-autoswitch/releases/download/v1.0.1/HDR.Auto-Switch_1.0.1_x64_en-US.msi)** | ~3.3 MB | Windows Installer package (MSI) |

---

### 💻 Compatibility
* **OS**: Windows 10 (20H2+) / Windows 11 (64-bit)
* **Display**: At least one HDR10 / VESA DisplayHDR capable monitor
