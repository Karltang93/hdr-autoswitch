# 🎮 HDR Auto-Switch for Windows

<div align="center">

![HDR Auto-Switch Banner](docs/screenshot.png)

**Automatic, zero-overhead HDR display switcher for Windows 10 and 11.**  
*No more manual `Win + Alt + B` before and after every gaming session.*

[![Windows](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/Soptik1290/hdr-autoswitch)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-FFC131?style=for-the-badge&logo=tauri&logoColor=black)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-Backend-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React 19](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black)](https://react.dev/)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)

[**Download Latest Release (.exe Installer)**](https://github.com/Soptik1290/hdr-autoswitch/releases/latest) • [**Report Bug**](https://github.com/Soptik1290/hdr-autoswitch/issues)

</div>

---

## ⚡ Why HDR Auto-Switch?

Windows HDR looks breathtaking in games and movies, but running desktop apps and web browsers in constant HDR often causes washed-out SDR colors, unnecessary power draw, and panel wear on OLED monitors.

**HDR Auto-Switch** runs quietly in your system tray and monitors window focus using native Windows OS events. The moment you launch or Alt+Tab into an HDR-enabled game, your monitor instantly engages HDR10 / Rec.2020. When you return to the desktop or browser, it gracefully switches back to SDR BT.709.

---

## ✨ Key Features

* ⚡ **True 0.0% CPU Overhead (No Polling):**  
  Unlike conventional tools that continuously poll running processes via `while True` or `setInterval`, HDR Auto-Switch registers a zero-overhead OS event hook (`SetWinEventHook` with `EVENT_SYSTEM_FOREGROUND`). The CPU remains 99.99% asleep until a window focus event actually occurs.
* 🖥️ **Native Win32 DisplayConfig API:**  
  Controls display color profiles directly via the graphics driver pipeline (`QueryDisplayConfig` / `SetDisplayConfig`), operating independently of the Xbox Game Bar and without keyboard simulation.
* 🛡️ **Alt+Tab Flicker Protection (Debounce):**  
  Tabbing out of a game for 2 seconds to check Discord or Spotify won't trigger annoying display flickering. Configurable delay from 0 to 10 seconds.
* 👾 **Retro Cyberpunk UI with GSAP Glitch:**  
  Crafted with an authentic dark cyberpunk terminal aesthetic featuring monospace typography (`Kode Mono`), CRT scanlines, and GSAP SVG displacement glitch animations.
* 📚 **Built-in 949+ Game Database:**  
  Includes curated Native HDR titles (*Cyberpunk 2077, Black Myth: Wukong, Alan Wake 2, Elden Ring, Forza Horizon 5*), all 275+ official Windows Auto HDR titles from PCGamingWiki, media players, and 1-click online sync.
* 🎨 **Visual Game Library with Steam Artwork:**  
  Scans your Steam, Epic Games, and Windows installations automatically. Renders high-resolution 600x900 vertical poster artwork with HDR status badges.
* 🎯 **Per-Monitor Targeting:**  
  Choose to switch all connected HDR displays simultaneously or bind automatic HDR switching specifically to your primary OLED gaming monitor.
* 🌐 **Automatic Bilingual Localization:**  
  Automatically launches in **Czech** if Windows is set to Czech/Slovak, and in **English** everywhere else. Includes an instant 1-click `CZ` / `EN` toggle in the header and settings.
* 📌 **System Tray & Autostart:**  
  Closing the window minimizes the app to the system tray. Supports silent launch on Windows startup.

---

## 📦 Installation & Download

### Option 1: Pre-built Windows Installer (Recommended)
Download the latest Windows installer (`HDR-Auto-Switch-Setup.exe` or `.msi`) from the [**Releases Page**](https://github.com/Soptik1290/hdr-autoswitch/releases).

1. Run `HDR-Auto-Switch-Setup.exe`.
2. The application will launch with your custom display configuration.
3. Click **"Scan PC for Games"** on the My Games tab or let it detect games in real time.

---

## 🛠️ Tech Stack & Architecture

| Layer | Technology |
|---|---|
| **Framework** | [Tauri v2](https://v2.tauri.app/) (Lightweight, native Windows runtime) |
| **Backend** | Rust (`windows-rs` Win32 API, `tauri-plugin-autostart`, `tauri-plugin-notification`) |
| **Frontend** | React 19, TypeScript, Vite, Tailwind CSS v4 |
| **Motion & FX** | [GSAP](https://gsap.com/) (SVG displacement filters, text scramble, CRT scanlines) |
| **Icons & Font** | Lucide React & Google Font `Kode Mono` |

---

## 💻 Developer Quickstart

### Prerequisites
* [Node.js](https://nodejs.org/) (v20+ recommended)
* [Rust](https://www.rust-lang.org/) (stable toolchain)
* Windows 10 (build 17763+) or Windows 11 with HDR display

### Setup
```bash
# Clone the repository
git clone https://github.com/Soptik1290/hdr-autoswitch.git
cd hdr-autoswitch

# Install frontend dependencies
npm install

# Run in development mode (hot-reloading)
npm run tauri dev
```

### Production Build
To create the optimized Windows `.exe` installer and portable binary:
```bash
npm run tauri build
```
The output installers will be generated in:
`src-tauri/target/release/bundle/nsis/` and `src-tauri/target/release/`

---

## 📄 License

Distributed under the [MIT License](LICENSE).  
Developed with ❤️ for the PC gaming community.
