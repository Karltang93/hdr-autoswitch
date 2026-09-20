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
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub alternate_exes: Vec<String>,
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
        let key = clean_key(&emb.name);
        catalog_map
            .entry(key)
            .and_modify(|existing| {
                if existing.steam_id.is_none() && emb.steam_id.is_some() {
                    existing.steam_id = emb.steam_id.clone();
                }
                let emb_exe = emb.exe_name.to_lowercase();
                if !emb_exe.is_empty() && emb_exe != existing.exe_name.to_lowercase() && !existing.alternate_exes.contains(&emb_exe) {
                    existing.alternate_exes.push(emb_exe);
                }
                for alt in &emb.alternate_exes {
                    let alt_clean = alt.to_lowercase();
                    if !existing.alternate_exes.contains(&alt_clean) && alt_clean != existing.exe_name.to_lowercase() {
                        existing.alternate_exes.push(alt_clean);
                    }
                }
            })
            .or_insert(emb);
    }

    // 2. Merge additional entries from online sync cached on disk
    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(cached_entries) = serde_json::from_str::<Vec<CatalogEntry>>(&content) {
                for cached in cached_entries {
                    let key = clean_key(&cached.name);
                    catalog_map.entry(key).or_insert(cached);
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

pub fn save_to_cache(entries: &[CatalogEntry]) -> Result<(), String> {
    let cache_file = get_cache_path();
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Cannot create catalog cache: {error}"))?;
    }
    let json = serde_json::to_string_pretty(entries).map_err(|error| error.to_string())?;
    fs::write(&cache_file, json).map_err(|error| format!("Cannot save catalog cache: {error}"))?;
    let mut write_guard = CACHED_CATALOG
        .write()
        .map_err(|_| "Catalog cache lock is poisoned.".to_string())?;
    *write_guard = Some(entries.to_vec());
    Ok(())
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
            alternate_exes: entry.alternate_exes,
            steam_id: entry.steam_id,
            launcher: None,
        })
        .collect()
}


pub fn find_in_catalog(exe_name: &str) -> Option<CatalogEntry> {
    let catalog = get_full_catalog();
    let exe_clean = exe_name.to_lowercase();
    let exe_stem = exe_clean.trim_end_matches(".exe");

    // 1. Direct match with entry.exe_name or entry.alternate_exes
    if let Some(entry) = catalog.iter().find(|c| {
        c.exe_name.eq_ignore_ascii_case(&exe_clean)
            || c.alternate_exes.iter().any(|alt| alt.eq_ignore_ascii_case(&exe_clean))
    }) {
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
            cat_stem == stripped_stem
                || cat_clean == clean_stripped
                || c.alternate_exes.iter().any(|alt| {
                    let alt_stem = alt.to_lowercase();
                    alt_stem.trim_end_matches(".exe") == stripped_stem
                })
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
    let mut fetched = false;

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
        fetched = true;
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
                        steam_id: None,
                        alternate_exes: Vec::new(),
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
                    fetched |= parse_pcgw_autohdr_wikitext(wikitext, &mut catalog_map) > 0;
                }
            }
        }
    }

    // 3. Fallback: If both failed, try GitHub repository raw JSON
    if !fetched {
        let gh_url = "https://raw.githubusercontent.com/Soptik1290/hdr-autoswitch/main/database/hdr_games.json";
        if let Ok(res) = client
            .get(gh_url)
            .header("User-Agent", "HDR-AutoSwitch-App")
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(entries) = res.json::<Vec<CatalogEntry>>().await {
                    for entry in entries.into_iter().filter(valid_catalog_entry) {
                        fetched = true;
                        catalog_map.insert(clean_key(&entry.name), entry);
                    }
                }
            }
        }
    }

    let result = synced_catalog(fetched, catalog_map)?;
    save_to_cache(&result)?;
    Ok(result)
}

fn valid_catalog_entry(entry: &CatalogEntry) -> bool {
    !clean_key(&entry.name).is_empty()
        && !entry.exe_name.contains(['\\', '/'])
        && entry.exe_name.to_ascii_lowercase().ends_with(".exe")
        && entry.exe_name.len() > 4
}

fn synced_catalog(
    fetched: bool,
    catalog_map: HashMap<String, CatalogEntry>,
) -> Result<Vec<CatalogEntry>, String> {
    if !fetched {
        return Err("No online catalog source succeeded. The existing catalog was not replaced.".into());
    }

    let mut result: Vec<CatalogEntry> = catalog_map.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
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
                    if !clean_key(game_name).is_empty()
                        && matches!(supported_val, "true" | "hackable" | "limited" | "always on")
                    {
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

fn parse_pcgw_autohdr_wikitext(wikitext: &str, map: &mut HashMap<String, CatalogEntry>) -> usize {
    let mut recognized = 0;
    for line in wikitext.lines() {
        let Some(row) = line.trim().strip_prefix('|') else {
            continue;
        };
        let Some(link) = row.trim().strip_prefix("[[") else {
            continue;
        };
        let Some((inside, _)) = link.split_once("]]") else {
            continue;
        };
        let (page, label) = inside.split_once('|').unwrap_or((inside, inside));
        let game_name = label.trim();
        if clean_key(page).is_empty()
            || clean_key(game_name).is_empty()
            || ["file:", "image:", "category:", "template:", "help:"]
                .iter()
                .any(|prefix| page.trim().to_ascii_lowercase().starts_with(prefix))
        {
            continue;
        }
        recognized += 1;
        let key = clean_key(game_name);
        map.entry(key.clone()).or_insert_with(|| CatalogEntry {
            name: game_name.to_string(),
            exe_name: format!("{key}.exe"),
            hdr_type: HdrType::AutoHdr,
            support_tier: "autohdr".to_string(),
            notes: Some("Podporuje Microsoft Windows Auto HDR".to_string()),
            steam_id: None,
            alternate_exes: Vec::new(),
        });
    }
    recognized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_error_and_changed_format_pages_do_not_count_as_ingestion() {
        for input in [
            "", "   ", "<html>Service unavailable</html>",
            r#"{"error":{"code":"missingtitle"}}"#,
            "{{AutoHDR|New table format}}", "| no recognized game rows",
            "| [[File:HDR.png|Picture]]", "| [[Category:HDR|HDR games]]",
            "| [[|Missing page]]", "| [[Game|]]", "| [[...]]",
        ] {
            let mut catalog = HashMap::new();
            assert_eq!(parse_pcgw_autohdr_wikitext(input, &mut catalog), 0, "{input}");
            assert!(synced_catalog(false, catalog).is_err());
        }
    }

    #[test]
    fn valid_rows_count_even_when_the_catalog_already_contains_them() {
        let text = "{| class=\"wikitable\"\n| [[Game one]] || Yes\n|-\n| [[Game two|Game 2]] || Yes\n|}";
        let mut catalog = HashMap::new();
        assert_eq!(parse_pcgw_autohdr_wikitext(text, &mut catalog), 2);
        catalog.get_mut("gameone").unwrap().steam_id = Some("123".into());
        let recognized = parse_pcgw_autohdr_wikitext(text, &mut catalog);
        assert_eq!(recognized, 2);
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog["gameone"].steam_id.as_deref(), Some("123"));
        assert!(synced_catalog(recognized > 0, catalog).is_ok());
    }

    #[test]
    fn an_existing_catalog_does_not_turn_a_failed_fetch_into_success() {
        let mut catalog = HashMap::new();
        parse_pcgw_autohdr_wikitext("| [[Existing game]]", &mut catalog);
        let recognized = parse_pcgw_autohdr_wikitext("A nonempty upstream error", &mut catalog);
        assert!(synced_catalog(recognized > 0, catalog).unwrap_err().contains("No online catalog source succeeded"));
    }

    #[test]
    fn fallback_requires_valid_entries_and_native_rows_require_known_support() {
        let mut rows = Vec::new();
        parse_pcgw_table_html(
            "<td class=\"field_Name\"><a>Game</a></td><td class=\"field_Supported\">new unknown value</td>",
            &mut rows,
        );
        assert!(rows.is_empty());
        parse_pcgw_table_html(
            "<td class=\"field_Name\"><a>Game</a></td><td class=\"field_Supported\">true</td>",
            &mut rows,
        );
        assert_eq!(rows, vec![("Game".into(), "true".into())]);
        let mut map = HashMap::new();
        parse_pcgw_autohdr_wikitext("| [[Fallback game]]", &mut map);
        let mut entry = map.remove("fallbackgame").unwrap();
        assert!(valid_catalog_entry(&entry));
        entry.exe_name = ".exe".into();
        assert!(!valid_catalog_entry(&entry));
        entry.exe_name = r"C:\game.exe".into();
        assert!(!valid_catalog_entry(&entry));
    }
}
