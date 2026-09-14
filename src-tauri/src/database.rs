use crate::config::{HdrApp, HdrType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub exe_name: String,
    pub hdr_type: HdrType,
    pub support_tier: String, // "native", "limited", "always_on", "manual_fix", "autohdr", "media"
    pub notes: Option<String>,
}

pub fn get_full_catalog() -> Vec<CatalogEntry> {
    let raw = vec![
        ("Forza Horizon 6", "forzahorizon6.exe", HdrType::Native, "native", "Nativní podpora HDR"),
        ("Battlefield 6", "bf6.exe", HdrType::Native, "native", "Nativní podpora HDR"),
        ("The Finals", "discovery.exe", HdrType::Native, "native", "Nativní Unreal Engine 5 HDR"),
        ("Enshrouded", "enshrouded.exe", HdrType::Native, "native", "Nativní HDR podpora"),
        ("Assetto Corsa", "acs.exe", HdrType::Native, "manual_fix", "HDR podpora přes Custom Shaders Patch"),
        ("Cyberpunk 2077", "cyberpunk2077.exe", HdrType::Native, "native", "Plná nativní HDR podpora"),
        ("Elden Ring", "eldenring.exe", HdrType::Native, "native", "Nativní podpora HDR10"),
        ("Alan Wake 2", "alanwake2.exe", HdrType::Native, "native", "Špičková nativní HDR kalibrace"),
        ("Baldur's Gate 3", "bg3.exe", HdrType::Native, "native", "Nativní HDR (Vulkan & DX11)"),
        ("Baldur's Gate 3 (DX11)", "bg3_dx11.exe", HdrType::Native, "native", "Nativní HDR"),
        ("Black Myth: Wukong", "b1-win64-shipping.exe", HdrType::Native, "native", "Nativní Unreal Engine 5 HDR"),
        ("God of War", "gow.exe", HdrType::Native, "native", "Nativní podpora"),
        ("God of War Ragnarok", "gowr.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Horizon Zero Dawn", "horizonzerodawn.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Horizon Forbidden West", "horizonforbiddenwest.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Red Dead Redemption 2", "rdr2.exe", HdrType::Native, "native", "Nativní Game & Cinematic HDR"),
        ("The Witcher 3: Wild Hunt", "witcher3.exe", HdrType::Native, "native", "Next-gen nativní HDR update"),
        ("Ghost of Tsushima", "ghostoftsushima.exe", HdrType::Native, "native", "Nativní podpora"),
        ("DOOM Eternal", "doometernalx64tk-vk.exe", HdrType::Native, "native", "Nativní idTech 7 HDR"),
        ("DOOM Eternal (D3D12)", "doometernalx64.exe", HdrType::Native, "native", "Nativní HDR"),
        ("Starfield", "starfield.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Microsoft Flight Simulator", "flightsimulator.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Forza Horizon 5", "forzahorizon5.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Forza Motorsport", "forzamotorsport.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Dead Space Remake", "deadspace.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Resident Evil 4 Remake", "re4.exe", HdrType::Native, "native", "RE Engine nativní HDR"),
        ("Resident Evil Village", "re8.exe", HdrType::Native, "native", "RE Engine nativní HDR"),
        ("Resident Evil 7", "re7.exe", HdrType::Native, "native", "RE Engine nativní HDR"),
        ("Resident Evil 2 Remake", "re2.exe", HdrType::Native, "native", "RE Engine nativní HDR"),
        ("Resident Evil 3 Remake", "re3.exe", HdrType::Native, "native", "RE Engine nativní HDR"),
        ("Call of Duty HQ", "cod.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Destiny 2", "destiny2.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Death Stranding", "ds.exe", HdrType::Native, "native", "Nativní Decima Engine HDR"),
        ("Final Fantasy VII Remake", "ff7remake_.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Marvel's Spider-Man Remastered", "spider-man.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Marvel's Spider-Man: Miles Morales", "milesmorales.exe", HdrType::Native, "native", "Nativní podpora"),
        ("The Last of Us Part I", "tloi.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Returnal", "returnal-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Lies of P", "lop-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Helldivers 2", "helldivers2.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Avatar: Frontiers of Pandora", "afop.exe", HdrType::Native, "native", "Snowdrop Engine nativní HDR"),
        ("Diablo IV", "diablo iv.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Star Wars Jedi: Survivor", "jedisurvivor.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Star Wars Jedi: Fallen Order", "starwarsjedifallenorder.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Assassin's Creed Mirage", "acmirage.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Assassin's Creed Valhalla", "acvalhalla.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Assassin's Creed Odyssey", "acodyssey.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Assassin's Creed Origins", "aco.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Hogwarts Legacy", "hogwartslegacy.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Shadow of the Tomb Raider", "sottr.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Rise of the Tomb Raider", "rottr.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Control", "control_dx12.exe", HdrType::Native, "native", "Nativní HDR v DX12"),
        ("Metro Exodus Enhanced Edition", "metroexodus.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Senua's Saga: Hellblade II", "hellblade2-win64-shipping.exe", HdrType::Native, "native", "Unreal Engine 5 nativní HDR"),
        ("Lords of the Fallen", "lotf2-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Monster Hunter: World", "monsterhunterworld.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Ratchet & Clank: Rift Apart", "riftapart.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Dragon's Dogma 2", "dd2.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Hitman World of Assassination", "hitman3.exe", HdrType::Native, "native", "Nativní podpora"),
        ("No Man's Sky", "nms.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Apex Legends", "r5apex.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Halo Infinite", "haloinfinite.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Remnant II", "remnant2-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Space Marine 2", "warhammer 40000 space marine 2 - retail.exe", HdrType::Native, "native", "Nativní podpora"),
        ("S.T.A.L.K.E.R. 2 Heart of Chornobyl", "stalker2-win64-shipping.exe", HdrType::Native, "native", "Nativní Unreal Engine 5 HDR"),
        ("Armored Core VI", "armoredcore6.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Deathloop", "deathloop.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Ghostwire: Tokyo", "ghostwire-tokyo.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Hi-Fi RUSH", "hifirush.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Kena: Bridge of Spirits", "kena-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("A Plague Tale: Requiem", "aplaguetalerequiem_x64.exe", HdrType::Native, "native", "Nativní podpora"),
        ("A Plague Tale: Innocence", "aplaguetaleinnocence_x64.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Days Gone", "daysgone.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Uncharted: Legacy of Thieves", "u4.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Uncharted: The Lost Legacy", "tll.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Gears 5", "gears5.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Sea of Thieves", "sotgame.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Far Cry 6", "farcry6.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Watch Dogs: Legion", "watchdogslegion.exe", HdrType::Native, "native", "Nativní podpora"),
        ("The Callisto Protocol", "thecallistoprotocol-win64-shipping.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Borderlands 3", "borderlands3.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Tiny Tina's Wonderlands", "wonderlands.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Battlefield 2042", "bf2042.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Battlefield V", "bfv.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Battlefield 1", "bf1.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Dying Light 2", "dyinglightgame_x64_rwdi.exe", HdrType::Native, "native", "Nativní podpora"),
        ("The First Descendant", "m1-win64-shipping.exe", HdrType::Native, "native", "Nativní Unreal Engine 5 HDR"),
        ("Need for Speed Heat", "needforspeedheat.exe", HdrType::Native, "native", "Nativní podpora"),
        ("Need for Speed Unbound", "needforspeedunbound.exe", HdrType::Native, "native", "Nativní podpora"),

        // Limited native support (Green check with asterisk)
        ("Sekiro: Shadows Die Twice", "sekiro.exe", HdrType::Native, "limited", "Omezená podpora (vyžaduje fullscreen v nativním rozlišení)"),
        ("Deus Ex: Mankind Divided", "dxmd.exe", HdrType::Native, "limited", "Vyžaduje DX11 režim pro stabilní HDR"),

        // Always on (Olive lock)
        ("Star Wars: Squadrons", "starwarssquadrons.exe", HdrType::Native, "always_on", "HDR je ve hře trvale zapnuto pokud je v OS aktivní"),

        // Requires manual fix (Blue wrench)
        ("NieR:Automata", "nierautomata.exe", HdrType::Native, "manual_fix", "Doporučen mód Special K (HDR retrofit)"),

        // Windows Auto HDR Supported Games (from PCGamingWiki Auto HDR list)
        ("The Elder Scrolls V: Skyrim SE", "skyrimse.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Fallout 4", "fallout4.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Dark Souls III", "darksoulsiii.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Dark Souls Remastered", "darksoulsremastered.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Grand Theft Auto V", "gta5.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Batman: Arkham Knight", "batmanak.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("BioShock Infinite", "bioshockinfinite.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Dishonored 2", "dishonored2.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Dishonored: Death of the Outsider", "dishonoreddot_x64.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Prey", "prey.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Titanfall 2", "titanfall2.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Subnautica", "subnautica.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Subnautica: Below Zero", "subnauticabelowzero.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Metal Gear Solid V: The Phantom Pain", "mgsvtpp.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Far Cry 5", "farcry5.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Kingdom Come: Deliverance", "kingdomcome.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Mass Effect 1 (LE)", "masseffect1.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Mass Effect 2 (LE)", "masseffect2.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Mass Effect 3 (LE)", "masseffect3.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Yakuza 0", "yakuza0.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Yakuza Kiwami 2", "yakuzakiwami2.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Yakuza: Like a Dragon", "yakuzalikeadragon.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Like a Dragon: Infinite Wealth", "likeadragoninfinitewealth.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Persona 5 Royal", "p5r.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Persona 3 Reload", "p3r.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Wolfenstein II: The New Colossus", "newcolossus_x64vk.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Hellblade: Senua's Sacrifice", "hellblade-win64-shipping.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Ready Or Not", "readyornot-win64-shipping.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Bodycam", "bodycam.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("BeamNG.drive", "beamng.drive.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Teardown", "teardown.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR ověřeno"),
        ("Road to Vostok", "rtv.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Deep Rock Galactic", "fsd-win64-shipping.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Rust", "rustclient.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),
        ("Once Human", "oncehuman.exe", HdrType::AutoHdr, "autohdr", "Windows 11 Auto HDR"),

        // Media Players
        ("mpv Media Player", "mpv.exe", HdrType::Media, "media", "Open-source přehrávač s podporou HDR passthrough"),
        ("MPC-HC (x64)", "mpc-hc64.exe", HdrType::Media, "media", "Přehrávač médií s podporou madVR / HDR"),
        ("MPC-HC (x86)", "mpc-hc.exe", HdrType::Media, "media", "Přehrávač médií s podporou madVR / HDR"),
        ("MPC-BE (x64)", "mpc-be64.exe", HdrType::Media, "media", "Přehrávač médií s podporou HDR"),
        ("MPC-BE (x86)", "mpc-be.exe", HdrType::Media, "media", "Přehrávač médií s podporou HDR"),
        ("PotPlayer (x64)", "potplayer64.exe", HdrType::Media, "media", "Přehrávač médií s podporou HDR"),
        ("PotPlayer (x86)", "potplayer.exe", HdrType::Media, "media", "Přehrávač médií s podporou HDR"),
        ("VLC Media Player", "vlc.exe", HdrType::Media, "media", "Přehrávač s podporou 10-bit HDR výstupu"),
        ("Kodi Media Center", "kodi.exe", HdrType::Media, "media", "Media center s podporou HDR10 passthrough"),
    ];

    raw.into_iter()
        .map(|(name, exe_name, hdr_type, support_tier, notes)| CatalogEntry {
            name: name.to_string(),
            exe_name: exe_name.to_lowercase(),
            hdr_type,
            support_tier: support_tier.to_string(),
            notes: Some(notes.to_string()),
        })
        .collect()
}

pub fn get_default_catalog() -> Vec<HdrApp> {
    get_full_catalog()
        .into_iter()
        .map(|entry| HdrApp {
            name: entry.name,
            exe_name: entry.exe_name,
            enabled: true,
            hdr_type: entry.hdr_type,
            path: None,
            alternate_exes: Vec::new(),
        })
        .collect()
}

pub async fn fetch_online_database() -> Result<Vec<CatalogEntry>, String> {
    let url = "https://raw.githubusercontent.com/Soptik1290/hdr-autoswitch/main/database/hdr_games.json";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .header("User-Agent", "HDR-AutoSwitch-App")
        .send()
        .await
        .map_err(|e| format!("Chyba při stahování online databáze: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Server vrátil stavový kód: {}", res.status()));
    }

    let entries: Vec<CatalogEntry> = res
        .json()
        .await
        .map_err(|e| format!("Chyba při zpracování JSON: {}", e))?;

    Ok(entries)
}
