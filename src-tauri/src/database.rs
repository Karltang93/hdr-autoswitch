use crate::config::{HdrApp, HdrType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineDatabaseEntry {
    pub name: String,
    pub exe_name: String,
    pub hdr_type: HdrType,
}

pub fn get_default_catalog() -> Vec<HdrApp> {
    let items = vec![
        // Native HDR Games
        ("Cyberpunk 2077", "cyberpunk2077.exe", HdrType::Native),
        ("Elden Ring", "eldenring.exe", HdrType::Native),
        ("Alan Wake 2", "alanwake2.exe", HdrType::Native),
        ("Baldur's Gate 3", "bg3.exe", HdrType::Native),
        ("Baldur's Gate 3 (DX11)", "bg3_dx11.exe", HdrType::Native),
        ("Black Myth: Wukong", "b1-win64-shipping.exe", HdrType::Native),
        ("God of War", "gow.exe", HdrType::Native),
        ("God of War Ragnarok", "gowr.exe", HdrType::Native),
        ("Horizon Zero Dawn", "horizonzerodawn.exe", HdrType::Native),
        ("Horizon Forbidden West", "horizonforbiddenwest.exe", HdrType::Native),
        ("Red Dead Redemption 2", "rdr2.exe", HdrType::Native),
        ("The Witcher 3: Wild Hunt", "witcher3.exe", HdrType::Native),
        ("Ghost of Tsushima", "ghostoftsushima.exe", HdrType::Native),
        ("DOOM Eternal", "doometernalx64tk-vk.exe", HdrType::Native),
        ("DOOM Eternal", "doometernalx64.exe", HdrType::Native),
        ("Starfield", "starfield.exe", HdrType::Native),
        ("Microsoft Flight Simulator", "flightsimulator.exe", HdrType::Native),
        ("Forza Horizon 5", "forzahorizon5.exe", HdrType::Native),
        ("Forza Motorsport", "forzamotorsport.exe", HdrType::Native),
        ("Dead Space Remake", "deadspace.exe", HdrType::Native),
        ("Resident Evil 4 Remake", "re4.exe", HdrType::Native),
        ("Resident Evil Village", "re8.exe", HdrType::Native),
        ("Resident Evil 7", "re7.exe", HdrType::Native),
        ("Resident Evil 2 Remake", "re2.exe", HdrType::Native),
        ("Resident Evil 3 Remake", "re3.exe", HdrType::Native),
        ("Call of Duty HQ", "cod.exe", HdrType::Native),
        ("Destiny 2", "destiny2.exe", HdrType::Native),
        ("Death Stranding", "ds.exe", HdrType::Native),
        ("Final Fantasy VII Remake", "ff7remake_.exe", HdrType::Native),
        ("Marvel's Spider-Man Remastered", "spider-man.exe", HdrType::Native),
        ("Marvel's Spider-Man: Miles Morales", "milesmorales.exe", HdrType::Native),
        ("The Last of Us Part I", "tloi.exe", HdrType::Native),
        ("Returnal", "returnal-win64-shipping.exe", HdrType::Native),
        ("Lies of P", "lop-win64-shipping.exe", HdrType::Native),
        ("Helldivers 2", "helldivers2.exe", HdrType::Native),
        ("Avatar: Frontiers of Pandora", "afop.exe", HdrType::Native),
        ("Diablo IV", "diablo iv.exe", HdrType::Native),
        ("Star Wars Jedi: Survivor", "jedisurvivor.exe", HdrType::Native),
        ("Star Wars Jedi: Fallen Order", "starwarsjedifallenorder.exe", HdrType::Native),
        ("Assassin's Creed Mirage", "acmirage.exe", HdrType::Native),
        ("Assassin's Creed Valhalla", "acvalhalla.exe", HdrType::Native),
        ("Assassin's Creed Odyssey", "acodyssey.exe", HdrType::Native),
        ("Assassin's Creed Origins", "aco.exe", HdrType::Native),
        ("Hogwarts Legacy", "hogwartslegacy.exe", HdrType::Native),
        ("Shadow of the Tomb Raider", "sottr.exe", HdrType::Native),
        ("Rise of the Tomb Raider", "rottr.exe", HdrType::Native),
        ("Control", "control_dx12.exe", HdrType::Native),
        ("Metro Exodus", "metroexodus.exe", HdrType::Native),
        ("Senua's Saga: Hellblade II", "hellblade2-win64-shipping.exe", HdrType::Native),
        ("Lords of the Fallen", "lotf2-win64-shipping.exe", HdrType::Native),
        ("Monster Hunter: World", "monsterhunterworld.exe", HdrType::Native),
        ("Ratchet & Clank: Rift Apart", "riftapart.exe", HdrType::Native),
        ("Dragon's Dogma 2", "dd2.exe", HdrType::Native),
        ("Hitman World of Assassination", "hitman3.exe", HdrType::Native),
        ("No Man's Sky", "nms.exe", HdrType::Native),
        ("Sekiro: Shadows Die Twice", "sekiro.exe", HdrType::Native),
        ("Apex Legends", "r5apex.exe", HdrType::Native),
        ("Halo Infinite", "haloinfinite.exe", HdrType::Native),
        ("Remnant II", "remnant2-win64-shipping.exe", HdrType::Native),
        ("Space Marine 2", "warhammer 40000 space marine 2 - retail.exe", HdrType::Native),
        ("S.T.A.L.K.E.R. 2", "stalker2-win64-shipping.exe", HdrType::Native),
        ("Armored Core VI", "armoredcore6.exe", HdrType::Native),
        ("Deathloop", "deathloop.exe", HdrType::Native),
        ("Ghostwire: Tokyo", "ghostwire-tokyo.exe", HdrType::Native),
        ("Hi-Fi RUSH", "hifirush.exe", HdrType::Native),
        ("Kena: Bridge of Spirits", "kena-win64-shipping.exe", HdrType::Native),
        ("A Plague Tale: Requiem", "aplaguetalerequiem_x64.exe", HdrType::Native),
        ("A Plague Tale: Innocence", "aplaguetaleinnocence_x64.exe", HdrType::Native),
        ("Days Gone", "daysgone.exe", HdrType::Native),
        ("Uncharted: Legacy of Thieves", "u4.exe", HdrType::Native),
        ("Uncharted: The Lost Legacy", "tll.exe", HdrType::Native),
        ("Gears 5", "gears5.exe", HdrType::Native),
        ("Sea of Thieves", "sotgame.exe", HdrType::Native),
        ("Far Cry 6", "farcry6.exe", HdrType::Native),
        ("Watch Dogs: Legion", "watchdogslegion.exe", HdrType::Native),
        ("The Callisto Protocol", "thecallistoprotocol-win64-shipping.exe", HdrType::Native),
        ("Borderlands 3", "borderlands3.exe", HdrType::Native),
        ("Tiny Tina's Wonderlands", "wonderlands.exe", HdrType::Native),
        ("Battlefield 2042", "bf2042.exe", HdrType::Native),
        ("Battlefield V", "bfv.exe", HdrType::Native),
        ("Battlefield 1", "bf1.exe", HdrType::Native),

        // Auto HDR Supported Games (from PCGamingWiki List of games that support Auto HDR)
        ("The Elder Scrolls V: Skyrim SE", "skyrimse.exe", HdrType::AutoHdr),
        ("Fallout 4", "fallout4.exe", HdrType::AutoHdr),
        ("Dark Souls III", "darksoulsiii.exe", HdrType::AutoHdr),
        ("Dark Souls Remastered", "darksoulsremastered.exe", HdrType::AutoHdr),
        ("Grand Theft Auto V", "gta5.exe", HdrType::AutoHdr),
        ("Batman: Arkham Knight", "batmanak.exe", HdrType::AutoHdr),
        ("BioShock Infinite", "bioshockinfinite.exe", HdrType::AutoHdr),
        ("Dishonored 2", "dishonored2.exe", HdrType::AutoHdr),
        ("Dishonored: Death of the Outsider", "dishonoreddot_x64.exe", HdrType::AutoHdr),
        ("Prey", "prey.exe", HdrType::AutoHdr),
        ("Titanfall 2", "titanfall2.exe", HdrType::AutoHdr),
        ("Subnautica", "subnautica.exe", HdrType::AutoHdr),
        ("Subnautica: Below Zero", "subnauticabelowzero.exe", HdrType::AutoHdr),
        ("NieR:Automata", "nierautomata.exe", HdrType::AutoHdr),
        ("Metal Gear Solid V: The Phantom Pain", "mgsvtpp.exe", HdrType::AutoHdr),
        ("Far Cry 5", "farcry5.exe", HdrType::AutoHdr),
        ("Kingdom Come: Deliverance", "kingdomcome.exe", HdrType::AutoHdr),
        ("Mass Effect 1 (LE)", "masseffect1.exe", HdrType::AutoHdr),
        ("Mass Effect 2 (LE)", "masseffect2.exe", HdrType::AutoHdr),
        ("Mass Effect 3 (LE)", "masseffect3.exe", HdrType::AutoHdr),
        ("Yakuza 0", "yakuza0.exe", HdrType::AutoHdr),
        ("Yakuza Kiwami 2", "yakuzakiwami2.exe", HdrType::AutoHdr),
        ("Yakuza: Like a Dragon", "yakuzalikeadragon.exe", HdrType::AutoHdr),
        ("Like a Dragon: Infinite Wealth", "likeadragoninfinitewealth.exe", HdrType::AutoHdr),
        ("Persona 5 Royal", "p5r.exe", HdrType::AutoHdr),
        ("Persona 3 Reload", "p3r.exe", HdrType::AutoHdr),
        ("Deus Ex: Mankind Divided", "dxmd.exe", HdrType::AutoHdr),
        ("Wolfenstein II: The New Colossus", "newcolossus_x64vk.exe", HdrType::AutoHdr),
        ("Hellblade: Senua's Sacrifice", "hellblade-win64-shipping.exe", HdrType::AutoHdr),
        ("Star Wars: Squadrons", "starwarssquadrons.exe", HdrType::AutoHdr),

        // Media Players
        ("mpv Media Player", "mpv.exe", HdrType::Media),
        ("MPC-HC (x64)", "mpc-hc64.exe", HdrType::Media),
        ("MPC-HC (x86)", "mpc-hc.exe", HdrType::Media),
        ("MPC-BE (x64)", "mpc-be64.exe", HdrType::Media),
        ("MPC-BE (x86)", "mpc-be.exe", HdrType::Media),
        ("PotPlayer (x64)", "potplayer64.exe", HdrType::Media),
        ("PotPlayer (x86)", "potplayer.exe", HdrType::Media),
        ("VLC Media Player", "vlc.exe", HdrType::Media),
        ("Kodi Media Center", "kodi.exe", HdrType::Media),
    ];

    items
        .into_iter()
        .map(|(name, exe_name, hdr_type)| HdrApp {
            name: name.to_string(),
            exe_name: exe_name.to_lowercase(),
            enabled: true,
            hdr_type,
            path: None,
        })
        .collect()
}

pub async fn fetch_online_database() -> Result<Vec<HdrApp>, String> {
    // Online community source or GitHub raw list
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

    let entries: Vec<OnlineDatabaseEntry> = res
        .json()
        .await
        .map_err(|e| format!("Chyba při zpracování JSON: {}", e))?;

    let apps = entries
        .into_iter()
        .map(|e| HdrApp {
            name: e.name,
            exe_name: e.exe_name.to_lowercase(),
            enabled: true,
            hdr_type: e.hdr_type,
            path: None,
        })
        .collect();

    Ok(apps)
}
