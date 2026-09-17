use crate::config::{HdrApp, HdrType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub exe_name: String,
    pub hdr_type: HdrType,
    pub support_tier: String, // "native", "limited", "always_on", "manual_fix", "autohdr", "media"
    pub notes: Option<String>,
}

static EMBEDDED_CATALOG_JSON: &str = include_str!("../catalog.json");

static CACHED_CATALOG: RwLock<Option<Vec<CatalogEntry>>> = RwLock::new(None);

fn get_cache_path() -> PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(app_data).join("HDRAutoSwitch").join("catalog_cache.json")
}

pub fn get_full_catalog() -> Vec<CatalogEntry> {
    if let Ok(read_guard) = CACHED_CATALOG.read() {
        if let Some(ref cat) = *read_guard {
            return cat.clone();
        }
    }

    let embedded: Vec<CatalogEntry> = serde_json::from_str(EMBEDDED_CATALOG_JSON).unwrap_or_default();

    // Read cache on disk and merge with embedded catalog
    let cache_file = get_cache_path();
    let mut catalog_map: HashMap<String, CatalogEntry> = HashMap::new();

    // 1. Put all embedded entries from current binary (always fresh & authoritative)
    for emb in embedded {
        catalog_map.insert(clean_key(&emb.name), emb);
    }

    // 2. Merge additional entries from online sync cached on disk
    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(cached_entries) = serde_json::from_str::<Vec<CatalogEntry>>(&content) {
                for cached in cached_entries {
                    catalog_map.entry(clean_key(&cached.name)).or_insert(cached);
                }
            }
        }
    }

    let mut entries: Vec<CatalogEntry> = catalog_map.into_values().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    if let Ok(mut write_guard) = CACHED_CATALOG.write() {
        *write_guard = Some(entries.clone());
    }

    entries
}

pub fn save_to_cache(entries: &[CatalogEntry]) {
    let cache_file = get_cache_path();
    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = fs::write(&cache_file, json);
    }
    if let Ok(mut write_guard) = CACHED_CATALOG.write() {
        *write_guard = Some(entries.to_vec());
    }
}

#[allow(dead_code)]
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
            steam_id: None,
            launcher: None,
        })
        .collect()
}

pub fn find_in_catalog(exe_name: &str) -> Option<CatalogEntry> {
    let catalog = get_full_catalog();
    let exe_clean = exe_name.to_lowercase();
    let exe_stem = exe_clean.trim_end_matches(".exe");

    // 1. Direct match with entry.exe_name
    if let Some(entry) = catalog.iter().find(|c| c.exe_name.eq_ignore_ascii_case(&exe_clean)) {
        return Some(entry.clone());
    }

    // 2. Direct match with clean stem against entry.name (e.g. "forzahorizon5" == clean_key("Forza Horizon 5"))
    let clean_exe_alphanumeric: String = exe_stem.chars().filter(|c| c.is_alphanumeric()).collect();
    if clean_exe_alphanumeric.len() >= 3 {
        if let Some(entry) = catalog.iter().find(|c| {
            let cat_clean = clean_key(&c.name);
            cat_clean == clean_exe_alphanumeric
        }) {
            return Some(entry.clone());
        }
    }

    // 3. Unreal Engine & shipping prefixes/suffixes: "game-win64-shipping", "game_dx12", "game_vk"
    let stripped_stem = exe_stem
        .replace("-win64-shipping", "")
        .replace("_win64_shipping", "")
        .replace("-shipping", "")
        .replace("_dx12", "")
        .replace("_dx11", "")
        .replace("_vk", "");
    let clean_stripped: String = stripped_stem.chars().filter(|c| c.is_alphanumeric()).collect();
    if clean_stripped.len() >= 3 && clean_stripped != clean_exe_alphanumeric {
        if let Some(entry) = catalog.iter().find(|c| {
            let cat_exe_clean = c.exe_name.to_lowercase();
            let cat_stem = cat_exe_clean.trim_end_matches(".exe");
            let cat_clean = clean_key(&c.name);
            cat_stem == stripped_stem || cat_clean == clean_stripped
        }) {
            return Some(entry.clone());
        }
    }

    None
}

pub async fn fetch_online_database() -> Result<Vec<CatalogEntry>, String> {
    let current_catalog = get_full_catalog();
    let mut catalog_map: HashMap<String, CatalogEntry> = current_catalog
        .into_iter()
        .map(|entry| (clean_key(&entry.name), entry))
        .collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Try PCGamingWiki MediaWiki Cargo Query for HDR games
    let mut pcgw_fetched = Vec::new();
    for offset in [0, 500] {
        let cargo_query = format!(
            "{{{{#cargo_query:tables=Game,Video|join on=Game._pageID=Video._pageID|where=Video.HDR='true' OR Video.HDR='hackable' OR Video.HDR='always on' OR Video.HDR='limited'|fields=Game._pageName=Name,Video.HDR=Supported|limit=500|offset={}|format=table}}}}",
            offset
        );

        let params = [
            ("action", "parse"),
            ("text", &cargo_query),
            ("contentmodel", "wikitext"),
            ("format", "json"),
        ];

        if let Ok(res) = client
            .post("https://www.pcgamingwiki.com/w/api.php")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            )
            .form(&params)
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(json_data) = res.json::<serde_json::Value>().await {
                    if let Some(html) = json_data.pointer("/parse/text/*").and_then(|v| v.as_str()) {
                        parse_pcgw_table_html(html, &mut pcgw_fetched);
                    }
                }
            }
        }
    }

    if !pcgw_fetched.is_empty() {
        for (name, supported) in pcgw_fetched {
            let key = clean_key(&name);
            let (tier, hdr_type, notes) = match supported.as_str() {
                "hackable" => ("manual_fix", HdrType::Custom, "Vyžaduje úpravu / mod / Special K (PCGamingWiki)"),
                "limited" => ("limited", HdrType::Native, "Omezená nativní podpora HDR (PCGamingWiki)"),
                "always on" => ("always_on", HdrType::Native, "Trvale aktivní v enginu (PCGamingWiki)"),
                _ => ("native", HdrType::Native, "Nativní HDR podpora (PCGamingWiki)"),
            };

            catalog_map
                .entry(key)
                .and_modify(|existing| {
                    existing.support_tier = tier.to_string();
                })
                .or_insert_with(|| {
                    let clean_exe = name
                        .to_lowercase()
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect::<String>();
                    CatalogEntry {
                        name,
                        exe_name: format!("{}.exe", clean_exe),
                        hdr_type,
                        support_tier: tier.to_string(),
                        notes: Some(notes.to_string()),
                    }
                });
        }
    }

    // 2. Fetch PCGamingWiki Windows Auto HDR games page
    let autohdr_params = [
        ("action", "parse"),
        ("page", "List_of_games_that_support_Auto_HDR"),
        ("prop", "wikitext"),
        ("format", "json"),
    ];

    if let Ok(res) = client
        .post("https://www.pcgamingwiki.com/w/api.php")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        )
        .form(&autohdr_params)
        .send()
        .await
    {
        if res.status().is_success() {
            if let Ok(json_data) = res.json::<serde_json::Value>().await {
                if let Some(wikitext) = json_data.pointer("/parse/wikitext/*").and_then(|v| v.as_str()) {
                    parse_pcgw_autohdr_wikitext(wikitext, &mut catalog_map);
                }
            }
        }
    }

    // 3. Fallback: If both failed, try GitHub repository raw JSON
    if catalog_map.len() < 200 {
        let gh_url = "https://raw.githubusercontent.com/Soptik1290/hdr-autoswitch/main/database/hdr_games.json";
        if let Ok(res) = client
            .get(gh_url)
            .header("User-Agent", "HDR-AutoSwitch-App")
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(entries) = res.json::<Vec<CatalogEntry>>().await {
                    for entry in entries {
                        catalog_map.insert(clean_key(&entry.name), entry);
                    }
                }
            }
        }
    }

    let mut result: Vec<CatalogEntry> = catalog_map.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    save_to_cache(&result);

    Ok(result)
}

fn clean_key(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn parse_pcgw_table_html(html: &str, out: &mut Vec<(String, String)>) {
    let name_tag = "<td class=\"field_Name\">";
    let supp_tag = "<td class=\"field_Supported\">";

    let mut rest = html;
    while let Some(name_pos) = rest.find(name_tag) {
        let after_name = &rest[name_pos + name_tag.len()..];
        if let Some(a_end) = after_name.find("</a>") {
            let before_a_end = &after_name[..a_end];
            let game_name = if let Some(last_gt) = before_a_end.rfind('>') {
                &before_a_end[last_gt + 1..]
            } else {
                before_a_end
            }
            .trim();

            if let Some(supp_pos) = after_name.find(supp_tag) {
                let after_supp = &after_name[supp_pos + supp_tag.len()..];
                if let Some(td_end) = after_supp.find("</td>") {
                    let supported_val = after_supp[..td_end].trim();
                    if !game_name.is_empty() {
                        out.push((game_name.to_string(), supported_val.to_string()));
                    }
                    rest = &after_supp[td_end..];
                    continue;
                }
            }
        }
        rest = after_name;
    }
}

fn parse_pcgw_autohdr_wikitext(wikitext: &str, map: &mut HashMap<String, CatalogEntry>) {
    for line in wikitext.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.contains("[[") && trimmed.contains("]]") {
            if let Some(start_bracket) = trimmed.find("[[") {
                if let Some(end_bracket) = trimmed[start_bracket..].find("]]") {
                    let inside = &trimmed[start_bracket + 2..start_bracket + end_bracket];
                    let game_name = if let Some(pipe_pos) = inside.find('|') {
                        &inside[pipe_pos + 1..]
                    } else {
                        inside
                    }
                    .trim();

                    if !game_name.is_empty() && !game_name.starts_with("File:") {
                        let key = clean_key(game_name);
                        let clean_exe = game_name
                            .to_lowercase()
                            .chars()
                            .filter(|c| c.is_alphanumeric())
                            .collect::<String>();

                        map.entry(key).or_insert_with(|| CatalogEntry {
                            name: game_name.to_string(),
                            exe_name: format!("{}.exe", clean_exe),
                            hdr_type: HdrType::AutoHdr,
                            support_tier: "autohdr".to_string(),
                            notes: Some("Podporuje Microsoft Windows Auto HDR".to_string()),
                        });
                    }
                }
            }
        }
    }
}
