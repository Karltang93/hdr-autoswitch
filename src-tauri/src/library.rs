use crate::config::{AppConfig, HdrApp};

fn same_game(existing: &HdrApp, item: &HdrApp) -> bool {
    if existing.exe_name.eq_ignore_ascii_case(&item.exe_name)
        || existing.name.eq_ignore_ascii_case(&item.name)
    {
        return true;
    }
    if matches!((&existing.steam_id, &item.steam_id), (Some(left), Some(right)) if left != right) {
        return false;
    }
    std::iter::once(&existing.exe_name)
        .chain(&existing.alternate_exes)
        .any(|exe| {
            std::iter::once(&item.exe_name)
                .chain(&item.alternate_exes)
                .any(|other| exe.eq_ignore_ascii_case(other))
        })
}

fn merge_aliases(existing: &mut HdrApp, item: &HdrApp) {
    for exe in std::iter::once(&item.exe_name).chain(&item.alternate_exes) {
        if !existing.exe_name.eq_ignore_ascii_case(exe)
            && !existing
                .alternate_exes
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(exe))
        {
            existing.alternate_exes.push(exe.to_lowercase());
        }
    }
}

pub fn import_games(config: &mut AppConfig, detected: Vec<HdrApp>) -> Result<(), String> {
    for mut item in detected {
        validate_app(&item)?;
        item.enabled = true;
        if let Some(existing) = config.apps.iter_mut().find(|app| same_game(app, &item)) {
            merge_aliases(existing, &item);
            if item.path.is_some() {
                existing.path = item.path;
            }
            if item.launcher.is_some() {
                existing.launcher = item.launcher;
            }
            if item.steam_id.is_some() {
                existing.steam_id = item.steam_id;
            }
            existing.enabled = true;
        } else {
            config.apps.push(item);
        }
    }
    Ok(())
}

pub fn enrich_existing(config: &mut AppConfig, detected: &[HdrApp]) -> bool {
    let mut changed = false;
    for item in detected {
        if let Some(existing) = config.apps.iter_mut().find(|app| same_game(app, item)) {
            let before = existing.clone();
            merge_aliases(existing, item);
            if existing.steam_id.is_none() {
                existing.steam_id.clone_from(&item.steam_id);
            }
            if existing.launcher.is_none() {
                existing.launcher.clone_from(&item.launcher);
            }
            changed |= *existing != before;
        }
    }
    changed
}

pub fn validate_app(app: &HdrApp) -> Result<(), String> {
    if app.name.trim().is_empty()
        || app.exe_name.trim().is_empty()
        || app.exe_name.contains(['\\', '/'])
        || !app.exe_name.to_ascii_lowercase().ends_with(".exe")
    {
        return Err("A game needs a name and an executable filename ending in .exe.".into());
    }
    Ok(())
}

pub fn add_app(config: &mut AppConfig, mut app: HdrApp) -> Result<(), String> {
    validate_app(&app)?;
    app.exe_name = app.exe_name.trim().to_lowercase();
    if let Some(existing) = config
        .apps
        .iter_mut()
        .find(|entry| same_game(entry, &app))
    {
        merge_aliases(existing, &app);
        existing.name = app.name;
        existing.enabled = app.enabled;
        existing.hdr_type = app.hdr_type;
        if app.path.is_some() {
            existing.path = app.path;
        }
        if app.steam_id.is_some() {
            existing.steam_id = app.steam_id;
        }
        if app.launcher.is_some() {
            existing.launcher = app.launcher;
        }
    } else {
        config.apps.push(app);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HdrType;

    fn game(name: &str, exe: &str) -> HdrApp {
        HdrApp {
            name: name.into(),
            exe_name: exe.into(),
            enabled: true,
            hdr_type: HdrType::Native,
            path: None,
            alternate_exes: Vec::new(),
            steam_id: None,
            launcher: None,
        }
    }

    #[test]
    fn background_enrichment_cannot_add_enable_or_overwrite_user_choices() {
        let mut existing = game("My game", "game.exe");
        existing.enabled = false;
        existing.path = Some("D:\\MyChoice\\game.exe".into());
        existing.steam_id = Some("user-choice".into());
        existing.launcher = Some("Custom".into());
        let mut detected = game("My game", "game.exe");
        detected.path = Some("C:\\Other\\game.exe".into());
        detected.steam_id = Some("scanner-choice".into());
        detected.launcher = Some("Steam".into());
        detected.alternate_exes = vec!["game-dx12.exe".into()];
        let mut config = AppConfig::default();
        config.apps = vec![existing.clone()];
        enrich_existing(&mut config, &[detected, game("Removed", "removed.exe")]);
        assert_eq!(config.apps.len(), 1);
        let actual = &config.apps[0];
        assert!(!actual.enabled);
        assert_eq!(actual.path, existing.path);
        assert_eq!(actual.steam_id, existing.steam_id);
        assert_eq!(actual.launcher, existing.launcher);
        assert_eq!(actual.alternate_exes, vec!["game-dx12.exe"]);
    }

    #[test]
    fn explicit_import_may_update_path_and_enable_existing_entry() {
        let mut existing = game("My game", "game.exe");
        existing.enabled = false;
        existing.path = Some("D:\\Old\\game.exe".into());
        let mut detected = game("My game", "game-dx12.exe");
        detected.path = Some("E:\\Moved\\game-dx12.exe".into());
        let mut config = AppConfig::default();
        config.apps = vec![existing];
        import_games(&mut config, vec![detected.clone()]).unwrap();
        assert_eq!(config.apps.len(), 1);
        assert!(config.apps[0].enabled);
        assert_eq!(config.apps[0].path, detected.path);
        assert_eq!(config.apps[0].alternate_exes, vec!["game-dx12.exe"]);
    }

    #[test]
    fn add_does_not_erase_previously_enriched_metadata() {
        let mut existing = game("My game", "game.exe");
        existing.path = Some("D:\\Games\\game.exe".into());
        existing.steam_id = Some("123".into());
        let mut config = AppConfig::default();
        config.apps = vec![existing.clone()];
        add_app(&mut config, game("My game", "GAME.EXE")).unwrap();
        assert_eq!(config.apps.len(), 1);
        assert_eq!(config.apps[0].path, existing.path);
        assert_eq!(config.apps[0].steam_id, existing.steam_id);
    }

    #[test]
    fn primary_and_alias_overlap_is_symmetric_for_every_library_entry_point() {
        let mut existing = game("User title", "main.exe");
        existing.enabled = false;
        existing.path = Some(r"D:\Chosen\main.exe".into());
        existing.alternate_exes = vec!["renderer.exe".into()];
        for (primary, aliases) in [
            ("MAIN.EXE", vec![]),
            ("RENDERER.EXE", vec![]),
            ("new.exe", vec!["MAIN.EXE"]),
            ("new.exe", vec!["RENDERER.EXE"]),
        ] {
            let mut incoming = game("Catalog title", primary);
            incoming.alternate_exes = aliases.into_iter().map(str::to_owned).collect();
            incoming.launcher = Some("Steam".into());
            assert!(same_game(&existing, &incoming));
            assert!(same_game(&incoming, &existing));
            for operation in 0..3 {
                let mut config = AppConfig::default();
                config.apps = vec![existing.clone()];
                match operation {
                    0 => add_app(&mut config, incoming.clone()).unwrap(),
                    1 => import_games(&mut config, vec![incoming.clone()]).unwrap(),
                    _ => { assert!(enrich_existing(&mut config, &[incoming.clone()])); }
                }
                assert_eq!(config.apps.len(), 1);
                assert_eq!(config.apps[0].path, existing.path);
                assert_eq!(config.apps[0].exe_name, existing.exe_name);
                assert_eq!(config.apps[0].enabled, operation != 2);
                assert_eq!(config.apps[0].launcher.as_deref(), Some("Steam"));
            }
        }
    }

    #[test]
    fn unrelated_titles_and_conflicting_game_ids_are_not_merged() {
        let left = game("Game", "game.exe");
        let right = game("Game Deluxe", "game-deluxe.exe");
        assert!(!same_game(&left, &right));
        assert!(!same_game(&right, &left));
        let mut left = left;
        let mut right = right;
        left.steam_id = Some("100".into());
        right.steam_id = Some("200".into());
        left.alternate_exes.push("launcher.exe".into());
        right.alternate_exes.push("launcher.exe".into());
        assert!(!same_game(&left, &right));
        assert!(!same_game(&right, &left));
    }

    #[test]
    fn enrichment_reports_no_change_for_unknown_games_and_existing_metadata() {
        let existing = game("Known game", "known.exe");
        let mut config = AppConfig::default();
        config.apps = vec![existing.clone()];
        assert!(!enrich_existing(&mut config, &[game("Unknown", "unknown.exe")]));
        assert!(!enrich_existing(&mut config, &[existing]));
        let mut detected = game("Known game", "alternate.exe");
        detected.launcher = Some("Steam".into());
        assert!(enrich_existing(&mut config, &[detected.clone()]));
        assert!(!enrich_existing(&mut config, &[detected]));
    }
}
