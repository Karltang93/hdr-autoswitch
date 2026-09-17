# HDR Auto-Switch v1.0.5 – Release Notes

Verze **v1.0.5** přináší zásadní opravy detekce nativních HDR her (řešení nahlášeného problému v GitHub Issue), propojení s oficiálními Steam AppID a masivní rozšíření herní databáze o 2 nové ověřené zdroje bez duplicit.

---

## 🚀 Hlavní novinky a opravy

### 1. 🎯 Přímé Steam AppID párování (Strategie 0)
- Každá hra instalovaná přes Steam má svůj manifest `appmanifest_{appid}.acf`.
- V databázi je nově evidováno přes **330 oficiálních Steam AppID**.
- Detekce her ze Steamu (např. *Silent Hill 2*, *Resident Evil 7*, *Borderlands GOTY Enhanced*, *Baldur's Gate 3*, *Cyberpunk 2077*, *Witcher 3*, *Black Myth: Wukong* atd.) je nyní **100% okamžitá a neprůstřelná** – aplikace již není odkázána pouze na odhadování názvů složek nebo binárek.
- Skener automaticky najde reálný spustitelný soubor na disku a nastaví jej jako primární.

### 2. 🛠️ Oprava reálných `.exe` a podpora alternativních binárek (`alternate_exes`)
- Původní Cargo export z PCGamingWiki generoval syntetické názvy `.exe` (např. `alanwakeremastered.exe` namísto `game_f_x64_eos.exe`).
- Do databáze byly doplněny skutečné produkční názvy a alternativní spustitelné soubory pro desítky her:
  - **Alan Wake Remastered**: `game_f_x64_eos.exe` (+ `alanwakeremastered.exe`)
  - **Silent Hill 2**: `shproto-win64-shipping.exe` (+ `shproto.exe`, `silenthill2.exe`)
  - **Borderlands: Game of the Year Enhanced**: `borderlandsgoty.exe` (+ `borderlandsgameoftheyearenhanced.exe`, označeno jako Native HDR)
  - **Resident Evil 7: Biohazard**: `re7.exe` (+ `re7trial.exe`, `residentevil7biohazard.exe`)
  - **Resident Evil 2, 3, 4, Village**: `re2.exe`, `re3.exe`, `re4.exe`, `re8.exe`
  - **Baldur's Gate 3**: `bg3.exe` (+ `bg3_dx11.exe`)
  - A desítky dalších (*Dead Space*, *Forza Horizon 5*, *God of War*, *Tekken 8*, *Diablo IV*, *Street Fighter 6*, *Dead Island 2*, *Helldivers 2*, *Spider-Man 2*...).
- Opravena chyba v `database.rs`, kdy mergování v paměti přemazávalo alternativní `.exe` soubory stejné hry. Nyní jsou všechny alternativní varianty uchovány v `alternate_exes` a zohledněny při vyhledávání.

### 3. 🌐 Integrace dvou nových ověřených zdrojů bez duplicit
- **Steam Curator "HDR Games"**: Integrováno všech 323 doporučených her s přesnými AppID, recenzemi a odlišením nativního HDR od modifikací (RenoDX/Special K).
- **HDR Gamer list**: Integrováno 208 her s ověřenou nativní HDR kalibrací a nastavením z `hdrgamer.com`.
- **PCGamingWiki**: Všechna data z PCGW zůstávají plně zachována.
- **Nulové duplicity**: Databáze byla deduplikována a sjednocena pod kanonické názvy. Celkový počet unikátních her vzrostl z 959 na **1 096 her**.

### 4. 🈳 Podpora dvoujazyčných a datovaných názvů
- **Bilingvní Steam názvy**: Capcom hry na Steamu používající lomítka (např. `"RESIDENT EVIL 7 biohazard / BIOHAZARD 7 resident evil"`) jsou nyní korektně rozpoznány a spárovány.
- **Letopočty u remaků**: Názvy s rokem v závorkách (např. `"Silent Hill 2 (2024)"` vs `"SILENT HILL 2"`, `"Alone in the Dark (2024)"`) jsou očištěny a spolehlivě propojeny.

---

## 📦 Stažení a instalace
- `HDR-AutoSwitch_1.0.5_x64-setup.exe` (NSIS instalátor)
- `HDR-AutoSwitch_1.0.5_x64_en-US.msi` (MSI balíček)
