import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = (name) => readFileSync(new URL(`../src-tauri/src/${name}.rs`, import.meta.url), 'utf8');

test('all automatic discovery paths use shared authority, not catalog/title/stem heuristics', () => {
  const scanner = source('scanner');
  const automaticScanner = scanner.slice(scanner.indexOf('fn match_and_insert_game('), scanner.indexOf('// String & Title Helpers'));
  assert.match(automaticScanner, /automatic_authority::resolve/);
  assert.doesNotMatch(automaticScanner, /find_in_catalog|is_title_match|pick_best|collect_exes/);
  assert.doesNotMatch(scanner, /fn pick_best_primary_exe|fn collect_exes/);
  const monitor = source('monitor_hook');
  const enrollment = monitor.slice(monitor.indexOf('    fn enroll('), monitor.indexOf('    fn ensure_watcher('));
  assert.match(enrollment, /automatic_authority::resolve\(&crate::database::get_full_catalog\(\), None\)/);
  assert.doesNotMatch(enrollment, /find_in_catalog|is_title_match/);
});

test('startup enrichment and scanner defaults cannot bypass provider authority or auto-detect', () => {
  const background = source('background');
  assert.match(background, /enrich_verified_metadata/);
  assert.doesNotMatch(background, /library::enrich_existing/);
  assert.match(background, /mutate_if_changed/);
  assert.match(source('commands'), /scan_installed_games\(auto_detect\)/);
  assert.match(source('scanner'), /game.enabled &= auto_detect/);
  const library = source('library');
  const enrichment = library.slice(library.indexOf('pub fn enrich_verified_metadata('), library.indexOf('pub fn validate_app('));
  assert.doesNotMatch(enrichment, /merge_aliases|same_game|\.enabled\s*=|\.path\s*=|\.exe_name\s*=/);
  assert.match(enrichment, /existing.exe_name.eq_ignore_ascii_case/);
});
