use crate::config::{HdrApp, HdrType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorefrontProvider {
    Steam,
    Xbox,
}

/// Authored catalog data only; no installation, package identity, or runtime evidence is stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorefrontBinding {
    pub provider: StorefrontProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(default)]
    pub game_executables: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_executables: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorefrontLookupError {
    AmbiguousBindings,
    InvalidProductId,
    InvalidExecutable,
    DuplicateExecutable,
    ExcludedGameExecutable,
}

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub storefronts: Vec<StorefrontBinding>,
    // None adapts authored rows; source boundaries set a snapshot (Some([]) for online-only rows).
    // Never restored from disk, where suggestions could otherwise gain implicit Steam authority.
    #[serde(skip)]
    storefront_authority: Option<Vec<StorefrontBinding>>,
}

impl CatalogEntry {
    fn legacy_steam_binding(&self) -> Option<StorefrontBinding> {
        self.steam_id.as_ref().map(|product_id| StorefrontBinding {
            provider: StorefrontProvider::Steam,
            product_id: Some(product_id.clone()),
            game_executables: vec![self.exe_name.clone()],
            excluded_executables: Vec::new(),
        })
    }

    fn authoritative_bindings(&self) -> Vec<StorefrontBinding> {
        if let Some(bindings) = &self.storefront_authority {
            return bindings.clone();
        }
        let mut bindings = self.storefronts.clone();
        if !bindings.iter().any(|binding| binding.provider == StorefrontProvider::Steam) {
            bindings.extend(self.legacy_steam_binding());
        }
        bindings
    }

    /// Explicit Steam data suppresses the legacy adapter, even for another product or an empty
    /// executable list. Global alternate_exes are not provider authority. Consume get_full_catalog
    /// for source-checked entries, not directly deserialized online records.
    pub fn storefront_binding(
        &self,
        provider: StorefrontProvider,
        product_id: Option<&str>,
    ) -> Result<Option<StorefrontBinding>, StorefrontLookupError> {
        find_storefront_binding(std::slice::from_ref(self), provider, product_id)
            .map(|found| found.map(|(_, binding)| binding))
    }
}

fn normalized_executables(names: &[String]) -> Result<Vec<String>, StorefrontLookupError> {
    let mut normalized: Vec<String> = names.iter().map(|name| name.trim().to_lowercase()).collect();
    if normalized.iter().any(|name| {
        name.len() <= 4 || !name.ends_with(".exe") || name.contains(['\\', '/', ':'])
    }) {
        return Err(StorefrontLookupError::InvalidExecutable);
    }
    normalized.sort();
    if normalized.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(StorefrontLookupError::DuplicateExecutable);
    }
    Ok(normalized)
}

impl StorefrontBinding {
    fn normalized(mut self) -> Result<Self, StorefrontLookupError> {
        if let Some(product_id) = &mut self.product_id {
            *product_id = product_id.trim().to_string();
            if product_id.is_empty() {
                return Err(StorefrontLookupError::InvalidProductId);
            }
        }
        self.game_executables = normalized_executables(&self.game_executables)?;
        self.excluded_executables = normalized_executables(&self.excluded_executables)?;
        if self.game_executables.iter().any(|name| self.excluded_executables.contains(name)) {
            return Err(StorefrontLookupError::ExcludedGameExecutable);
        }
        Ok(self)
    }
}

/// Pure provider/product lookup. None leaves the product unconstrained; multiple candidates
/// (including identical duplicate rows) are ambiguous, never resolved by list order.
/// Names are normalized on returned copies only. Product IDs remain provider-local and case-sensitive.
pub fn find_storefront_binding<'a>(
    catalog: &'a [CatalogEntry],
    provider: StorefrontProvider,
    product_id: Option<&str>,
) -> Result<Option<(&'a CatalogEntry, StorefrontBinding)>, StorefrontLookupError> {
    let product_id = product_id.map(str::trim);
    if product_id == Some("") {
        return Err(StorefrontLookupError::InvalidProductId);
    }
    let mut candidates = catalog.iter().flat_map(|entry| {
        entry.authoritative_bindings().into_iter().filter_map(move |binding| {
            (binding.provider == provider
                && product_id.is_none_or(|id| binding.product_id.as_deref().map(str::trim) == Some(id)))
                .then_some((entry, binding))
        })
    });
    let Some((entry, binding)) = candidates.next() else {
        return Ok(None);
    };
    if candidates.next().is_some() {
        return Err(StorefrontLookupError::AmbiguousBindings);
    }
    binding.normalized().map(|binding| Some((entry, binding)))
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

    let cache_file = get_cache_path();
    let cached = match fs::read_to_string(cache_file) {
        Ok(content) => match serde_json::from_str::<Vec<CatalogEntry>>(&content) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("Cannot parse catalog cache; using embedded catalog: {error}");
                Vec::new()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            eprintln!("Cannot read catalog cache; using embedded catalog: {error}");
            Vec::new()
        }
    };
    let entries = merge_catalog(embedded_catalog(), cached);

    if let Ok(mut write_guard) = CACHED_CATALOG.write() {
        *write_guard = Some(entries.clone());
    }

    entries
}

fn embedded_catalog() -> Vec<CatalogEntry> {
    serde_json::from_str(EMBEDDED_CATALOG_JSON).expect("Embedded catalog must be valid")
}

fn merge_catalog(embedded: Vec<CatalogEntry>, cached: Vec<CatalogEntry>) -> Vec<CatalogEntry> {
    let mut catalog_map: HashMap<String, CatalogEntry> = HashMap::new();
    let mut legacy_bindings: HashMap<String, Vec<StorefrontBinding>> = HashMap::new();

    // Capture legacy authority before the existing display/alias merge can change its identity.
    for emb in embedded {
        let key = clean_key(&emb.name);
        legacy_bindings.entry(key.clone()).or_default().extend(emb.legacy_steam_binding());
        catalog_map
            .entry(key)
            .and_modify(|existing| {
                existing.storefronts.extend(emb.storefronts.clone());
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

    for (key, entry) in &mut catalog_map {
        let mut bindings = entry.storefronts.clone();
        if !bindings.iter().any(|binding| binding.provider == StorefrontProvider::Steam) {
            bindings.extend(legacy_bindings.remove(key).unwrap_or_default());
        }
        entry.storefront_authority = Some(bindings);
    }

    for mut entry in cached {
        restrict_storefront_authority(&mut entry, None);
        catalog_map.entry(clean_key(&entry.name)).or_insert(entry);
    }

    let mut entries: Vec<CatalogEntry> = catalog_map.into_values().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn restrict_storefront_authority(entry: &mut CatalogEntry, embedded: Option<&CatalogEntry>) {
    entry.storefronts = embedded.map(|source| source.storefronts.clone()).unwrap_or_default();
    entry.storefront_authority = Some(
        embedded.map(CatalogEntry::authoritative_bindings).unwrap_or_default(),
    );
}

// Also used before populating the in-memory cache: sync must not bypass the disk-load boundary.
fn with_embedded_authority(mut entries: Vec<CatalogEntry>) -> Vec<CatalogEntry> {
    let embedded: HashMap<String, CatalogEntry> = merge_catalog(embedded_catalog(), Vec::new())
        .into_iter()
        .map(|entry| (clean_key(&entry.name), entry))
        .collect();
    for entry in &mut entries {
        restrict_storefront_authority(entry, embedded.get(&clean_key(&entry.name)));
    }
    entries
}

pub fn save_to_cache(entries: &[CatalogEntry]) -> Result<(), String> {
    let entries = with_embedded_authority(entries.to_vec());
    let cache_file = get_cache_path();
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Cannot create catalog cache: {error}"))?;
    }
    let json = serde_json::to_string_pretty(&entries).map_err(|error| error.to_string())?;
    fs::write(&cache_file, json).map_err(|error| format!("Cannot save catalog cache: {error}"))?;
    let mut write_guard = CACHED_CATALOG
        .write()
        .map_err(|_| "Catalog cache lock is poisoned.".to_string())?;
    *write_guard = Some(entries);
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
                        storefronts: Vec::new(),
                        storefront_authority: None,
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

    let mut result = with_embedded_authority(catalog_map.into_values().collect());
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
            storefronts: Vec::new(),
            storefront_authority: None,
        });
    }
    recognized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_entry() -> CatalogEntry {
        serde_json::from_str(r#"{
            "name": "Example game", "exe_name": "Legacy.EXE", "hdr_type": "native",
            "support_tier": "native", "notes": null, "steam_id": "123",
            "alternate_exes": ["global.exe"]
        }"#).unwrap()
    }

    fn binding(
        provider: StorefrontProvider,
        product_id: Option<&str>,
        games: &[&str],
        excluded: &[&str],
    ) -> StorefrontBinding {
        StorefrontBinding {
            provider,
            product_id: product_id.map(str::to_string),
            game_executables: games.iter().map(|value| value.to_string()).collect(),
            excluded_executables: excluded.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn legacy_catalog_decodes_and_serializes_without_new_fields() {
        let entry = legacy_entry();
        assert!(entry.storefronts.is_empty());
        assert_eq!(entry.name, "Example game");
        assert_eq!(entry.hdr_type, HdrType::Native);
        assert_eq!(entry.alternate_exes, ["global.exe"]);
        let value = serde_json::to_value(&entry).unwrap();
        assert_eq!(value, serde_json::json!({
            "name": "Example game", "exe_name": "Legacy.EXE", "hdr_type": "native",
            "support_tier": "native", "notes": null, "steam_id": "123",
            "alternate_exes": ["global.exe"]
        }));
        assert_eq!(
            entry.storefront_binding(StorefrontProvider::Steam, Some("123")).unwrap(),
            Some(binding(StorefrontProvider::Steam, Some("123"), &["legacy.exe"], &[])),
        );
        assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Ok(None));

        let entry: CatalogEntry = serde_json::from_str(r#"{
            "name": "Old", "exe_name": "old.exe", "hdr_type": "native", "support_tier": "native"
        }"#).unwrap();
        assert!(entry.steam_id.is_none());
        assert!(entry.notes.is_none());
        assert!(entry.alternate_exes.is_empty());
        assert!(entry.storefronts.is_empty());
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None), Ok(None));
    }

    #[test]
    fn explicit_steam_overrides_legacy_without_borrowing_xbox_or_global_aliases() {
        let mut entry = legacy_entry();
        let steam = binding(StorefrontProvider::Steam, Some("456"), &["steam.exe"], &[]);
        let xbox = binding(StorefrontProvider::Xbox, None, &["xbox.exe"], &["helper.exe"]);
        entry.storefronts = vec![steam.clone(), xbox.clone()];
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None), Ok(Some(steam)));
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, Some("123")), Ok(None));
        assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Ok(Some(xbox)));
        assert_eq!(entry.alternate_exes, ["global.exe"]);
        assert_eq!(entry.exe_name, "Legacy.EXE");

        entry.storefronts[0].game_executables.clear();
        assert!(entry.storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap()
            .game_executables.is_empty());
        entry.storefronts.remove(0);
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap()
            .game_executables, ["legacy.exe"]);
    }

    #[test]
    fn binding_defaults_and_normalization_do_not_rewrite_authored_json() {
        let minimal: StorefrontBinding = serde_json::from_str(r#"{"provider":"xbox"}"#).unwrap();
        assert_eq!(minimal, binding(StorefrontProvider::Xbox, None, &[], &[]));
        assert_eq!(serde_json::to_value(minimal).unwrap(), serde_json::json!({
            "provider": "xbox", "game_executables": []
        }));
        assert!(serde_json::from_str::<StorefrontBinding>(r#"{"provider":"unknown"}"#).is_err());

        let mut entry = legacy_entry();
        entry.storefronts = vec![binding(
            StorefrontProvider::Xbox, Some(" local-id "), &[" Z.EXE ", "a.exe"], &["Helper.EXE"],
        )];
        let before = serde_json::to_value(&entry).unwrap();
        let expected = binding(StorefrontProvider::Xbox, Some("local-id"), &["a.exe", "z.exe"], &["helper.exe"]);
        assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, Some("local-id")), Ok(Some(expected.clone())));
        entry.storefronts[0].game_executables.reverse();
        assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, Some(" local-id ")), Ok(Some(expected)));
        entry.storefronts[0].game_executables.reverse();
        assert_eq!(serde_json::to_value(&entry).unwrap(), before);
        let round_trip: CatalogEntry = serde_json::from_value(before.clone()).unwrap();
        assert_eq!(serde_json::to_value(round_trip).unwrap(), before);
    }

    #[test]
    fn duplicate_basenames_and_positive_exclusion_conflicts_fail_closed() {
        let mut entry = legacy_entry();
        for (games, excluded, error) in [
            (vec!["GAME.exe", "game.EXE"], vec![], StorefrontLookupError::DuplicateExecutable),
            (vec!["game.exe"], vec!["HELPER.exe", "helper.EXE"], StorefrontLookupError::DuplicateExecutable),
            (vec!["GAME.exe"], vec!["game.EXE"], StorefrontLookupError::ExcludedGameExecutable),
        ] {
            entry.storefronts = vec![binding(StorefrontProvider::Xbox, None, &games, &excluded)];
            assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Err(error));
            entry.storefronts[0].game_executables.reverse();
            entry.storefronts[0].excluded_executables.reverse();
            assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Err(error));
        }
        for invalid in ["", ".exe", "game.dll", r"C:\game.exe", "dir/game.exe", "game:stream.exe"] {
            entry.storefronts = vec![binding(StorefrontProvider::Xbox, None, &[invalid], &[])];
            assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Err(StorefrontLookupError::InvalidExecutable));
        }
        entry.storefronts = vec![binding(StorefrontProvider::Steam, Some(" "), &["game.exe"], &[])];
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None), Err(StorefrontLookupError::InvalidProductId));
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, Some(" ")), Err(StorefrontLookupError::InvalidProductId));
    }

    #[test]
    fn duplicate_provider_products_are_ambiguous_in_either_order() {
        let mut entry = legacy_entry();
        for second_game in ["one.exe", "two.exe"] {
            entry.storefronts = vec![
                binding(StorefrontProvider::Steam, Some("123"), &["one.exe"], &[]),
                binding(StorefrontProvider::Steam, Some(" 123 "), &[second_game], &[]),
            ];
            for _ in 0..2 {
                assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, Some("123")), Err(StorefrontLookupError::AmbiguousBindings));
                assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None), Err(StorefrontLookupError::AmbiguousBindings));
                entry.storefronts.reverse();
            }
        }
        entry.storefronts = vec![
            binding(StorefrontProvider::Xbox, None, &["one.exe"], &[]),
            binding(StorefrontProvider::Xbox, None, &["two.exe"], &[]),
        ];
        assert_eq!(entry.storefront_binding(StorefrontProvider::Xbox, None), Err(StorefrontLookupError::AmbiguousBindings));
    }

    #[test]
    fn product_lookup_disambiguates_products_but_not_duplicate_catalog_rows() {
        let mut entry = legacy_entry();
        entry.storefronts = vec![
            binding(StorefrontProvider::Steam, Some("123"), &["game.exe"], &[]),
            binding(StorefrontProvider::Steam, Some("456"), &["game.exe"], &[]),
        ];
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, None), Err(StorefrontLookupError::AmbiguousBindings));
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, Some("456")).unwrap().unwrap()
            .product_id.as_deref(), Some("456"));
        assert_eq!(entry.storefront_binding(StorefrontProvider::Steam, Some("789")), Ok(None));
        let mut duplicate = entry.clone();
        duplicate.name = "Different title".into();
        let mut catalog = vec![entry, duplicate];
        for _ in 0..2 {
            assert_eq!(find_storefront_binding(&catalog, StorefrontProvider::Steam, Some("123")).unwrap_err(),
                StorefrontLookupError::AmbiguousBindings);
            catalog.reverse();
        }
    }

    #[test]
    fn embedded_title_merging_preserves_ambiguous_explicit_and_legacy_authority() {
        for explicit in [false, true] {
            let mut first = legacy_entry();
            if explicit {
                first.storefronts = vec![binding(StorefrontProvider::Steam, Some("123"), &["one.exe"], &[])];
            }
            let mut second = first.clone();
            second.name = "Example GAME!".into();
            second.exe_name = "two.exe".into();
            if explicit {
                second.storefronts[0].game_executables = vec!["two.exe".into()];
            }
            for embedded in [vec![first.clone(), second.clone()], vec![second, first]] {
                let merged = merge_catalog(embedded, Vec::new());
                assert_eq!(merged.len(), 1);
                assert_eq!(merged[0].storefront_binding(StorefrontProvider::Steam, Some("123")),
                    Err(StorefrontLookupError::AmbiguousBindings));
            }
        }
    }

    #[test]
    fn embedded_title_merging_does_not_reassign_legacy_exe_or_override_explicit_steam() {
        let legacy = legacy_entry();
        let mut suggestion = legacy.clone();
        suggestion.steam_id = None;
        suggestion.exe_name = "suggested.exe".into();
        let mut explicit = suggestion.clone();
        explicit.storefronts = vec![binding(StorefrontProvider::Steam, Some("456"), &[], &[])];
        for embedded in [vec![suggestion.clone(), legacy.clone()], vec![legacy.clone(), suggestion]] {
            let merged = merge_catalog(embedded, Vec::new());
            assert_eq!(merged[0].storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap()
                .game_executables, ["legacy.exe"]);
        }
        for embedded in [vec![legacy.clone(), explicit.clone()], vec![explicit, legacy]] {
            let merged = merge_catalog(embedded, Vec::new());
            assert_eq!(merged[0].storefront_binding(StorefrontProvider::Steam, Some("123")), Ok(None));
            assert_eq!(merged[0].storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap()
                .product_id.as_deref(), Some("456"));
        }
    }

    #[test]
    fn cached_records_cannot_override_or_add_explicit_or_implicit_authority() {
        let mut embedded = legacy_entry();
        embedded.storefronts = vec![binding(StorefrontProvider::Xbox, None, &["embedded.exe"], &["helper.exe"])];
        let mut cached = embedded.clone();
        cached.storefronts = vec![
            binding(StorefrontProvider::Xbox, None, &["cached.exe"], &[]),
            binding(StorefrontProvider::Steam, Some("456"), &["cached.exe"], &[]),
        ];
        cached.steam_id = Some("456".into());
        cached.exe_name = "cached.exe".into();
        let mut additional = cached.clone();
        additional.name = "Online suggestion".into();
        let mut legacy_only = additional.clone();
        legacy_only.name = "Online legacy suggestion".into();
        legacy_only.storefronts.clear();
        let catalog = merge_catalog(vec![embedded.clone()], vec![cached, additional, legacy_only]);
        let existing = catalog.iter().find(|entry| entry.name == embedded.name).unwrap();
        assert_eq!(existing.storefront_binding(StorefrontProvider::Xbox, None), embedded.storefront_binding(StorefrontProvider::Xbox, None));
        assert_eq!(existing.storefront_binding(StorefrontProvider::Steam, None), embedded.storefront_binding(StorefrontProvider::Steam, None));
        assert_eq!(existing.alternate_exes, ["global.exe"]);
        for suggestion in catalog.iter().filter(|entry| entry.name.starts_with("Online")) {
            assert_eq!(suggestion.exe_name, "cached.exe");
            assert_eq!(suggestion.steam_id.as_deref(), Some("456"));
            assert!(suggestion.storefronts.is_empty());
            assert_eq!(suggestion.storefront_binding(StorefrontProvider::Steam, None), Ok(None));
            assert_eq!(suggestion.storefront_binding(StorefrontProvider::Xbox, None), Ok(None));
            assert!(serde_json::to_value(suggestion).unwrap().get("storefront_authority").is_none());
        }
    }

    #[test]
    fn online_sync_and_memory_cache_boundary_restore_only_embedded_authority() {
        let embedded = embedded_catalog();
        let mut xbox = embedded.iter().find(|entry| entry.name == "Age of Empires III: Definitive Edition").unwrap().clone();
        let mut legacy = embedded.iter().find(|entry| entry.steam_id.as_deref() == Some("1466860")).unwrap().clone();
        for entry in [&mut xbox, &mut legacy] {
            entry.exe_name = "untrusted.exe".into();
            entry.steam_id = Some("untrusted-id".into());
            entry.notes = Some("Online display note".into());
            entry.storefronts = vec![binding(StorefrontProvider::Steam, Some("untrusted-id"), &["untrusted.exe"], &[])];
        }
        let suggestion = legacy_entry();
        let entries = vec![xbox, legacy, suggestion];
        let synced = synced_catalog(true, entries.iter().cloned().map(|entry| (clean_key(&entry.name), entry)).collect()).unwrap();
        let cache_ready = with_embedded_authority(entries);
        for catalog in [synced, cache_ready] {
            let xbox = catalog.iter().find(|entry| entry.name == "Age of Empires III: Definitive Edition").unwrap();
            assert_eq!(xbox.storefront_binding(StorefrontProvider::Steam, None), Ok(None));
            assert_eq!(xbox.storefront_binding(StorefrontProvider::Xbox, None).unwrap().unwrap()
                .game_executables, ["aoe3de.exe"]);
            let legacy = catalog.iter().find(|entry| entry.name == "Age of Empires IV").unwrap();
            assert_eq!(legacy.storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap(),
                binding(StorefrontProvider::Steam, Some("1466860"), &["ageofempiresiv.exe"], &[]));
            assert_eq!(legacy.exe_name, "untrusted.exe");
            assert_eq!(legacy.notes.as_deref(), Some("Online display note"));
            let suggestion = catalog.iter().find(|entry| entry.name == "Example game").unwrap();
            assert_eq!(suggestion.storefront_binding(StorefrontProvider::Steam, None), Ok(None));
            let reloaded: Vec<CatalogEntry> = serde_json::from_str(&serde_json::to_string(&catalog).unwrap()).unwrap();
            let merged = merge_catalog(embedded_catalog(), reloaded);
            assert_eq!(merged.iter().find(|entry| entry.name == "Example game").unwrap()
                .storefront_binding(StorefrontProvider::Steam, None), Ok(None));
        }
    }

    #[test]
    fn catalog_copies_match_and_aoe3_xbox_authority_leaves_steam_unresolved() {
        let source = include_str!("../../database/hdr_games.json");
        assert_eq!(serde_json::from_str::<serde_json::Value>(source).unwrap(),
            serde_json::from_str::<serde_json::Value>(EMBEDDED_CATALOG_JSON).unwrap());
        let catalog = merge_catalog(embedded_catalog(), Vec::new());
        let aoe3 = catalog.iter().find(|entry| entry.name == "Age of Empires III: Definitive Edition").unwrap();
        assert_eq!(aoe3.exe_name, "ageofempiresiiidefinitiveedition.exe");
        assert!(aoe3.steam_id.is_none());
        assert!(aoe3.alternate_exes.is_empty());
        assert_eq!(aoe3.storefront_binding(StorefrontProvider::Steam, None), Ok(None));
        assert_eq!(aoe3.storefront_binding(StorefrontProvider::Xbox, None).unwrap().unwrap(),
            binding(StorefrontProvider::Xbox, None, &["aoe3de.exe"], &["gamelaunchhelper.exe"]));
        assert!(find_in_catalog("aoe3de.exe").is_none());
        assert!(find_in_catalog("gamelaunchhelper.exe").is_none());
        let aoe4 = catalog.iter().find(|entry| entry.steam_id.as_deref() == Some("1466860")).unwrap();
        assert!(aoe4.storefronts.is_empty());
        assert_eq!(aoe4.storefront_binding(StorefrontProvider::Steam, None).unwrap().unwrap()
            .game_executables, ["ageofempiresiv.exe"]);
    }

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
