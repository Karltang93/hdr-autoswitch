//! Bounded, declaration-only evidence from two known MicrosoftGame.config locations.
//!
//! This deliberately supports a conservative XML 1.0 subset: UTF-8 (with an optional
//! BOM), an optional initial XML declaration, unqualified ASCII names, attributes,
//! text, and comments. DTDs, custom entities, namespaces, CDATA, and other processing
//! instructions are unsupported. Predefined entities and valid numeric character
//! references are decoded once. Unknown schema elements are checked, not interpreted.
//! Input is capped at 64 KiB, 1,024 nodes (including text, comments, and the XML
//! declaration), 32 element levels, 64 attributes per element, and 64 declarations.
//!
//! Before probing either config, shared root validation rejects nonabsolute roots,
//! network/device paths, and reparse ancestors. Its Windows GetDriveTypeW check also
//! rejects mapped network drives. The cooperative 500 ms budget cannot interrupt an
//! in-progress OS call. No executable is opened, selected, or run; shared authority
//! must independently validate and select from the returned declarations.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::automatic_authority::{checked_root, local_file, normalize_relative_exe};

const MAX_BYTES: usize = 64 * 1024;
const MAX_NODES: usize = 1024;
const MAX_DEPTH: usize = 32;
const MAX_ATTRIBUTES: usize = 64;
const MAX_DECLARATIONS: usize = 64;
const TIME_BUDGET: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    MissingConfig,
    AmbiguousConfigs,
    ConfigIo,
    UnsafeConfigPath,
    TooLarge,
    TimedOut,
    UnsupportedEncoding,
    UnsupportedXml,
    MalformedXml,
    TooManyNodes,
    TooDeep,
    TooManyAttributes,
    TooManyDeclarations,
    InvalidDeclaration,
    DuplicateDeclaration,
    NoDeclarations,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingConfig => "MicrosoftGame.config is absent from both known locations",
            Self::AmbiguousConfigs => "MicrosoftGame.config exists at both root and Content",
            Self::ConfigIo => "could not inspect, open, or read MicrosoftGame.config",
            Self::UnsafeConfigPath => "configuration requires a safe local root and regular file",
            Self::TooLarge => "MicrosoftGame.config exceeds 64 KiB",
            Self::TimedOut => "MicrosoftGame.config inspection exceeded its time budget",
            Self::UnsupportedEncoding => "only UTF-8 encoded XML is supported",
            Self::UnsupportedXml => "MicrosoftGame.config uses unsupported XML syntax",
            Self::MalformedXml => {
                "MicrosoftGame.config is not a balanced, well-formed Game document"
            }
            Self::TooManyNodes => "MicrosoftGame.config exceeds the XML node limit",
            Self::TooDeep => "MicrosoftGame.config exceeds the XML nesting limit",
            Self::TooManyAttributes => {
                "MicrosoftGame.config exceeds the per-element attribute limit"
            }
            Self::TooManyDeclarations => {
                "MicrosoftGame.config exceeds the executable declaration limit"
            }
            Self::InvalidDeclaration => {
                "Executable needs a case-sensitive Name with a safe relative exe path"
            }
            Self::DuplicateDeclaration => "Executable declarations normalize to the same path",
            Self::NoDeclarations => "Game/ExecutableList contains no Executable declarations",
        })
    }
}

impl std::error::Error for ConfigError {}

/// Read only root/MicrosoftGame.config or root/Content/MicrosoftGame.config.
///
/// Paths are normalized relative to the installation root, not the config directory.
/// All distinct declarations, including helpers, are returned in document order.
/// Neither a partial document nor an ambiguous or inaccessible config is evidence.
pub fn declarations(root: &Path) -> Result<Vec<String>, ConfigError> {
    let started = Instant::now();
    budget(started)?;
    let root_validation = checked_root(root);
    budget(started)?;
    // Retain the input spelling: canonical Windows paths can have device prefixes.
    let _ = root_validation.map_err(|_| ConfigError::UnsafeConfigPath)?;

    let locations = [
        (
            root.join("MicrosoftGame.config"),
            "MicrosoftGame.config",
            "",
        ),
        (
            root.join("Content").join("MicrosoftGame.config"),
            "Content\\MicrosoftGame.config",
            "content\\",
        ),
    ];
    let mut selected = None;
    for (path, relative, prefix) in &locations {
        budget(started)?;
        if !prefix.is_empty() {
            let content = root.join("Content");
            match fs::symlink_metadata(&content) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(_) => return Err(ConfigError::ConfigIo),
                Ok(_) => {
                    checked_root(&content).map_err(|_| ConfigError::UnsafeConfigPath)?;
                    budget(started)?;
                }
            }
        }
        let metadata = fs::symlink_metadata(path);
        budget(started)?;
        match metadata {
            Ok(_) => {
                if selected.is_some() {
                    return Err(ConfigError::AmbiguousConfigs);
                }
                selected = Some((*relative, *prefix));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(ConfigError::ConfigIo),
        }
    }
    let (relative, prefix) = selected.ok_or(ConfigError::MissingConfig)?;
    let safe_path = local_file(root, relative);
    budget(started)?;
    let safe_path = safe_path.map_err(|_| ConfigError::UnsafeConfigPath)?;
    let file = File::open(safe_path);
    budget(started)?;
    let file = file.map_err(|_| ConfigError::ConfigIo)?;
    let metadata = file.metadata();
    budget(started)?;
    let metadata = metadata.map_err(|_| ConfigError::ConfigIo)?;
    if !metadata.is_file() {
        return Err(ConfigError::UnsafeConfigPath);
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err(ConfigError::TooLarge);
    }

    // The extra byte catches growth after metadata; take bounds actual bytes read.
    let mut reader = file.take(MAX_BYTES as u64 + 1);
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut chunk = [0_u8; 4096];
    loop {
        budget(started)?;
        let read = reader.read(&mut chunk);
        budget(started)?;
        let count = match read {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ConfigError::ConfigIo),
        };
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_BYTES {
            return Err(ConfigError::TooLarge);
        }
    }
    parse_document(&bytes, prefix, started)
}

fn budget(started: Instant) -> Result<(), ConfigError> {
    if started.elapsed() >= TIME_BUDGET {
        Err(ConfigError::TimedOut)
    } else {
        Ok(())
    }
}

fn xml_character(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{a}' | '\u{d}'
            | '\u{20}'..='\u{d7ff}'
            | '\u{e000}'..='\u{fffd}'
            | '\u{10000}'..='\u{10ffff}'
    )
}

fn xml_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn parse_document(
    bytes: &[u8],
    prefix: &str,
    started: Instant,
) -> Result<Vec<String>, ConfigError> {
    budget(started)?;
    if bytes.len() > MAX_BYTES {
        return Err(ConfigError::TooLarge);
    }
    let input = std::str::from_utf8(bytes).map_err(|_| ConfigError::UnsupportedEncoding)?;
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    if !input.chars().all(xml_character) {
        return Err(ConfigError::MalformedXml);
    }
    Parser {
        input,
        position: 0,
        stack: Vec::new(),
        nodes: 0,
        seen_root: false,
        declarations: Vec::new(),
        prefix,
        started,
    }
    .parse()
}

type Attributes<'a> = Vec<(&'a str, String)>;

struct Parser<'a> {
    input: &'a str,
    position: usize,
    stack: Vec<&'a str>,
    nodes: usize,
    seen_root: bool,
    declarations: Vec<String>,
    prefix: &'a str,
    started: Instant,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.input[self.position..]
    }

    fn consume(&mut self, value: &str) -> bool {
        if self.rest().starts_with(value) {
            self.position += value.len();
            true
        } else {
            false
        }
    }

    fn space(&mut self) -> bool {
        let before = self.position;
        while self
            .input
            .as_bytes()
            .get(self.position)
            .copied()
            .is_some_and(xml_space)
        {
            self.position += 1;
        }
        self.position != before
    }

    fn node(&mut self) -> Result<(), ConfigError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            Err(ConfigError::TooManyNodes)
        } else {
            Ok(())
        }
    }

    fn name(&mut self) -> Result<&'a str, ConfigError> {
        let start = self.position;
        let first = *self
            .input
            .as_bytes()
            .get(start)
            .ok_or(ConfigError::MalformedXml)?;
        if first == b':' || !first.is_ascii() {
            return Err(ConfigError::UnsupportedXml);
        }
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Err(ConfigError::MalformedXml);
        }
        self.position += 1;
        while let Some(byte) = self.input.as_bytes().get(self.position) {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
                self.position += 1;
            } else {
                break;
            }
        }
        if self.input.as_bytes().get(self.position) == Some(&b':') {
            return Err(ConfigError::UnsupportedXml);
        }
        Ok(&self.input[start..self.position])
    }

    fn quoted(&mut self) -> Result<&'a str, ConfigError> {
        let quote = *self
            .input
            .as_bytes()
            .get(self.position)
            .ok_or(ConfigError::MalformedXml)?;
        if !matches!(quote, b'\'' | b'"') {
            return Err(ConfigError::MalformedXml);
        }
        self.position += 1;
        let end = self
            .rest()
            .find(char::from(quote))
            .ok_or(ConfigError::MalformedXml)?;
        let raw = &self.rest()[..end];
        if raw.contains('<') {
            return Err(ConfigError::MalformedXml);
        }
        self.position += end + 1;
        Ok(raw)
    }

    fn attributes(&mut self, declaration: bool) -> Result<(Attributes<'a>, bool), ConfigError> {
        let mut attributes: Attributes<'a> = Vec::new();
        loop {
            budget(self.started)?;
            let separated = self.space();
            if declaration {
                if self.consume("?>") {
                    return Ok((attributes, false));
                }
            } else {
                if self.consume("/>") {
                    return Ok((attributes, true));
                }
                if self.consume(">") {
                    return Ok((attributes, false));
                }
            }
            if !separated {
                return Err(ConfigError::MalformedXml);
            }
            if attributes.len() == MAX_ATTRIBUTES {
                return Err(ConfigError::TooManyAttributes);
            }
            let name = self.name()?;
            if name == "xmlns" {
                return Err(ConfigError::UnsupportedXml);
            }
            if attributes.iter().any(|(existing, _)| *existing == name) {
                return Err(ConfigError::MalformedXml);
            }
            self.space();
            if !self.consume("=") {
                return Err(ConfigError::MalformedXml);
            }
            self.space();
            let raw = self.quoted()?;
            if declaration && raw.contains('&') {
                return Err(ConfigError::MalformedXml);
            }
            attributes.push((name, decode(raw, true)?));
        }
    }

    fn xml_declaration(&mut self) -> Result<(), ConfigError> {
        self.node()?;
        self.position += "<?xml".len();
        if !self
            .rest()
            .as_bytes()
            .first()
            .copied()
            .is_some_and(xml_space)
        {
            return Err(ConfigError::UnsupportedXml);
        }
        let (attributes, _) = self.attributes(true)?;
        if attributes
            .first()
            .map(|(name, value)| (*name, value.as_str()))
            != Some(("version", "1.0"))
        {
            return Err(ConfigError::UnsupportedXml);
        }
        let mut index = 1;
        if let Some(("encoding", value)) = attributes.get(index) {
            if !value.eq_ignore_ascii_case("UTF-8") {
                return Err(ConfigError::UnsupportedEncoding);
            }
            index += 1;
        }
        if let Some(("standalone", value)) = attributes.get(index) {
            if value != "yes" && value != "no" {
                return Err(ConfigError::UnsupportedXml);
            }
            index += 1;
        }
        if index != attributes.len() {
            return Err(ConfigError::UnsupportedXml);
        }
        Ok(())
    }

    fn comment(&mut self) -> Result<(), ConfigError> {
        self.node()?;
        self.position += "<!--".len();
        let end = self.rest().find("-->").ok_or(ConfigError::MalformedXml)?;
        let body = &self.rest()[..end];
        if body.contains("--") || body.ends_with('-') {
            return Err(ConfigError::MalformedXml);
        }
        self.position += end + "-->".len();
        Ok(())
    }

    fn element(&mut self) -> Result<(), ConfigError> {
        self.node()?;
        if self.stack.len() >= MAX_DEPTH {
            return Err(ConfigError::TooDeep);
        }
        self.position += 1;
        let name = self.name()?;
        let (attributes, empty) = self.attributes(false)?;
        if self.stack.is_empty() {
            if self.seen_root || name != "Game" {
                return Err(ConfigError::MalformedXml);
            }
            self.seen_root = true;
        }
        if name == "Executable"
            && self.stack.len() == 2
            && self.stack[0] == "Game"
            && self.stack[1] == "ExecutableList"
        {
            if self.declarations.len() == MAX_DECLARATIONS {
                return Err(ConfigError::TooManyDeclarations);
            }
            let value = attributes
                .iter()
                .find(|(attribute, _)| *attribute == "Name")
                .map(|(_, value)| value)
                .ok_or(ConfigError::InvalidDeclaration)?;
            let relative =
                normalize_relative_exe(value).map_err(|_| ConfigError::InvalidDeclaration)?;
            let relative = format!("{}{relative}", self.prefix);
            if self.declarations.contains(&relative) {
                return Err(ConfigError::DuplicateDeclaration);
            }
            self.declarations.push(relative);
        }
        if !empty {
            self.stack.push(name);
        }
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), ConfigError> {
        self.position += "</".len();
        let name = self.name()?;
        self.space();
        if !self.consume(">") || self.stack.pop() != Some(name) {
            return Err(ConfigError::MalformedXml);
        }
        Ok(())
    }

    fn text(&mut self) -> Result<(), ConfigError> {
        self.node()?;
        let length = self.rest().find('<').unwrap_or(self.rest().len());
        let text = &self.rest()[..length];
        if text.contains("]]>") {
            return Err(ConfigError::MalformedXml);
        }
        if self.stack.is_empty() {
            if !text.bytes().all(xml_space) {
                return Err(ConfigError::MalformedXml);
            }
        } else {
            decode(text, false)?;
        }
        self.position += length;
        Ok(())
    }

    fn parse(mut self) -> Result<Vec<String>, ConfigError> {
        if self.rest().starts_with("<?xml") {
            self.xml_declaration()?;
        }
        while !self.rest().is_empty() {
            budget(self.started)?;
            if self.rest().starts_with("<!--") {
                self.comment()?;
            } else if self.rest().starts_with("<!") || self.rest().starts_with("<?") {
                return Err(ConfigError::UnsupportedXml);
            } else if self.rest().starts_with("</") {
                self.end_element()?;
            } else if self.rest().starts_with('<') {
                self.element()?;
            } else {
                self.text()?;
            }
        }
        budget(self.started)?;
        if !self.seen_root || !self.stack.is_empty() {
            return Err(ConfigError::MalformedXml);
        }
        if self.declarations.is_empty() {
            return Err(ConfigError::NoDeclarations);
        }
        Ok(self.declarations)
    }
}

fn decode(mut remaining: &str, attribute: bool) -> Result<String, ConfigError> {
    let mut output = String::with_capacity(remaining.len());
    while !remaining.is_empty() {
        if let Some(reference) = remaining.strip_prefix('&') {
            let end = reference.find(';').ok_or(ConfigError::MalformedXml)?;
            let reference_name = &reference[..end];
            let character = match reference_name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "apos" => '\'',
                "quot" => '"',
                numeric if numeric.starts_with('#') => {
                    let (digits, radix) = match numeric.strip_prefix("#x") {
                        Some(digits) => (digits, 16),
                        None => (&numeric[1..], 10),
                    };
                    if digits.is_empty()
                        || !digits.bytes().all(|byte| {
                            if radix == 16 {
                                byte.is_ascii_hexdigit()
                            } else {
                                byte.is_ascii_digit()
                            }
                        })
                    {
                        return Err(ConfigError::MalformedXml);
                    }
                    u32::from_str_radix(digits, radix)
                        .ok()
                        .and_then(char::from_u32)
                        .filter(|character| xml_character(*character))
                        .ok_or(ConfigError::MalformedXml)?
                }
                _ => return Err(ConfigError::UnsupportedXml),
            };
            output.push(character);
            remaining = &reference[end + 1..];
        } else {
            let character = remaining.chars().next().ok_or(ConfigError::MalformedXml)?;
            remaining = &remaining[character.len_utf8()..];
            // XML attribute normalization precedes character-reference replacement.
            if attribute && character == '\r' {
                remaining = remaining.strip_prefix('\n').unwrap_or(remaining);
                output.push(' ');
            } else if attribute && matches!(character, '\n' | '\t') {
                output.push(' ');
            } else {
                output.push(character);
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn empty_fixture() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_config(root: &Path, relative: &str, bytes: &[u8]) {
        let path = relative
            .split('\\')
            .fold(root.to_path_buf(), |path, component| path.join(component));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn game(names: &[&str]) -> String {
        let executables = names
            .iter()
            .map(|name| format!(r#"<Executable Name="{name}" />"#))
            .collect::<String>();
        format!("<Game><ExecutableList>{executables}</ExecutableList></Game>")
    }

    fn inspect(xml: &str) -> Result<Vec<String>, ConfigError> {
        let root = empty_fixture();
        write_config(root.path(), "MicrosoftGame.config", xml.as_bytes());
        declarations(root.path())
    }

    #[test]
    fn aoe3_root_declaration_and_normal_schema_data() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <!-- local declaration evidence -->
            <Game configVersion="1">
              <Identity Name="Microsoft.AoE3DE" Version="1.0.0.0" />
              <ShellVisuals DefaultDisplayName="Age of Empires III" />
              <Properties><Description>Fish &amp; chips &lt;3</Description></Properties>
              <ExecutableList>
                <Executable Name="AoE3DE.exe" Id="Game" TargetDeviceFamily="PC" />
              </ExecutableList>
            </Game><!-- trailing comments are legal -->"#;
        assert_eq!(inspect(xml).unwrap(), ["aoe3de.exe"]);
    }

    #[test]
    fn content_declaration_is_relative_to_the_install_root() {
        let root = empty_fixture();
        write_config(
            root.path(),
            "Content\\MicrosoftGame.config",
            game(&["Bin\\AoE3DE.exe"]).as_bytes(),
        );
        assert_eq!(
            declarations(root.path()).unwrap(),
            ["content\\bin\\aoe3de.exe"]
        );
    }

    #[test]
    fn helpers_and_multiple_distinct_declarations_are_not_selected_or_filtered() {
        assert_eq!(
            inspect(&game(&[
                "Helpers\\CrashReport.exe",
                "AoE3DE.exe",
                "Bin\\Other.exe"
            ]))
            .unwrap(),
            ["helpers\\crashreport.exe", "aoe3de.exe", "bin\\other.exe"]
        );
        assert_eq!(
            inspect(&game(&["Helpers\\CrashReport.exe"])).unwrap(),
            ["helpers\\crashreport.exe"]
        );
        assert_eq!(
            inspect(&game(&["GameLaunchHelper.exe", "AoE3DE.exe"])).unwrap(),
            ["gamelaunchhelper.exe", "aoe3de.exe"]
        );
        let root = empty_fixture();
        write_config(
            root.path(),
            "Content\\MicrosoftGame.config",
            game(&["GameLaunchHelper.exe"]).as_bytes(),
        );
        assert_eq!(
            declarations(root.path()).unwrap(),
            ["content\\gamelaunchhelper.exe"]
        );
    }

    #[test]
    fn only_the_exact_case_sensitive_executable_location_is_evidence() {
        let xml = r#"<Game>
            <Executable Name="Outside.exe"/>
            <Other><ExecutableList><Executable Name="Nested.exe"/></ExecutableList></Other>
            <ExecutableList>
                <executable Name="Lowercase.exe"/>
                <Wrapper><Executable Name="Wrapped.exe"/></Wrapper>
                <Executable name="Ignored.exe" Name="AoE3DE.exe"></Executable>
            </ExecutableList>
        </Game>"#;
        assert_eq!(inspect(xml).unwrap(), ["aoe3de.exe"]);
        assert_eq!(
            inspect(
                r#"<Game><ExecutableList><Executable name="Game.exe"/></ExecutableList></Game>"#
            ),
            Err(ConfigError::InvalidDeclaration)
        );
        assert_eq!(inspect("<Game/>"), Err(ConfigError::NoDeclarations));
    }

    #[test]
    fn comments_and_decoded_text_cannot_introduce_declarations() {
        let inert = r#"<!-- &custom; <ExecutableList><Executable Name="Comment.exe"/></ExecutableList> -->
            <Description>&lt;ExecutableList&gt;&lt;Executable Name="Text.exe"/&gt;&lt;/ExecutableList&gt;</Description>"#;
        assert_eq!(
            inspect(&format!("<Game>{inert}</Game>")),
            Err(ConfigError::NoDeclarations)
        );
        assert_eq!(
            inspect(&game(&["Game.exe"]).replace("</Game>", &format!("{inert}</Game>"))).unwrap(),
            ["game.exe"]
        );
    }

    #[test]
    fn missing_and_unrecognized_locations_are_not_scanned() {
        let root = empty_fixture();
        assert_eq!(declarations(root.path()), Err(ConfigError::MissingConfig));
        for location in [
            "Other\\MicrosoftGame.config",
            "Content\\Nested\\MicrosoftGame.config",
        ] {
            write_config(root.path(), location, game(&["Game.exe"]).as_bytes());
        }
        assert_eq!(declarations(root.path()), Err(ConfigError::MissingConfig));
    }

    #[test]
    fn two_known_configs_are_ambiguous_even_when_one_is_invalid() {
        let root = empty_fixture();
        write_config(
            root.path(),
            "MicrosoftGame.config",
            game(&["AoE3DE.exe"]).as_bytes(),
        );
        write_config(root.path(), "Content\\MicrosoftGame.config", b"not XML");
        assert_eq!(
            declarations(root.path()),
            Err(ConfigError::AmbiguousConfigs)
        );
    }

    #[test]
    fn a_non_regular_config_is_not_read() {
        let root = empty_fixture();
        fs::create_dir(root.path().join("MicrosoftGame.config")).unwrap();
        assert_eq!(
            declarations(root.path()),
            Err(ConfigError::UnsafeConfigPath)
        );
    }

    #[test]
    fn byte_limit_is_inclusive() {
        let root = empty_fixture();
        let mut bytes = game(&["Game.exe"]).into_bytes();
        bytes.resize(MAX_BYTES, b' ');
        write_config(root.path(), "MicrosoftGame.config", &bytes);
        assert_eq!(declarations(root.path()).unwrap(), ["game.exe"]);
        bytes.push(b' ');
        write_config(root.path(), "MicrosoftGame.config", &bytes);
        assert_eq!(declarations(root.path()), Err(ConfigError::TooLarge));
    }

    #[test]
    fn malformed_content_anywhere_invalidates_all_declarations() {
        let valid = game(&["AoE3DE.exe"]);
        let malformed = [
            String::new(),
            "<Game>".to_owned(),
            "<Other/>".to_owned(),
            valid.replace("</ExecutableList>", "</Other>"),
            valid.replace("</Game>", ""),
            format!("{valid}<Game/>"),
            format!("{valid}trailing"),
            format!("leading{valid}"),
            format!("{valid}</Game>"),
            valid.replace("<Game>", "<Game broken>"),
            valid.replace("<Game>", "<Game a=\"1\"a=\"2\">"),
            valid.replace("<Game>", "<Game a=\"1\" a=\"2\">"),
            valid.replace("<Game>", "<Game a=unquoted>"),
            valid.replace("<Game>", "<Game a=\"<\">"),
            valid.replace("</Game>", "</ Game>"),
            valid.replace("</Game>", "</Game attribute='value'>"),
            valid.replace("</Game>", "</Game/>"),
            valid.replace("</Game>", "</game>"),
            valid.replace("</Game>", "]]></Game>"),
            valid.replace("</Game>", "<!-- invalid -- comment --></Game>"),
            valid.replace("</Game>", "<!-- invalid ---></Game>"),
            valid.replace("</Game>", "<!-- unterminated</Game>"),
            valid.replace("/>", "/ >"),
            valid.replace("</Game>", "<Broken></Game>"),
        ];
        for xml in malformed {
            assert!(inspect(&xml).is_err(), "unexpectedly accepted {xml:?}");
        }
    }

    #[test]
    fn no_truncated_document_prefix_can_return_partial_evidence() {
        let xml = "<?xml version='1.0' encoding='UTF-8'?><Game a='&quot;>'>\
            <ExecutableList><Executable Name='AoE3DE.exe'/></ExecutableList>\
            <Other>caf\u{e9} &lt;ok&gt;<!-- checked too --></Other></Game>";
        assert_eq!(inspect(xml).unwrap(), ["aoe3de.exe"]);
        for length in 0..xml.len() {
            assert!(
                parse_document(&xml.as_bytes()[..length], "", Instant::now()).is_err(),
                "truncated document of {length} bytes returned declarations"
            );
        }
    }

    #[test]
    fn dtd_entity_definitions_cdata_and_processing_instructions_fail_closed() {
        let valid = game(&["AoE3DE.exe"]);
        for forbidden in [
            "<!DOCTYPE Game>",
            "<!DOCTYPE Game SYSTEM 'file:///not-read'>",
            "<!DOCTYPE Game SYSTEM 'https://invalid.example/not-fetched'>",
            "<!DOCTYPE Game [<!ENTITY custom 'AoE3DE.exe'>]>",
            "<!ENTITY custom 'AoE3DE.exe'>",
            "<?custom data?>",
            "<?xml-stylesheet href='ignored'?>",
        ] {
            assert_eq!(
                inspect(&format!("{forbidden}{valid}")),
                Err(ConfigError::UnsupportedXml)
            );
        }
        for forbidden in ["<![CDATA[ignored]]>", "<?xml version='1.0'?>", "<?custom?>"] {
            assert_eq!(
                inspect(&valid.replace("</Game>", &format!("{forbidden}</Game>"))),
                Err(ConfigError::UnsupportedXml)
            );
        }
    }

    #[test]
    fn namespaces_are_unsupported_even_on_unrelated_elements() {
        let valid = game(&["AoE3DE.exe"]);
        for xml in [
            valid.replace("<Game>", "<Game xmlns=\"urn:game\">"),
            valid.replace("<Game>", "<Game xmlns=\"\">"),
            valid.replace("<Game>", "<Game xmlns:g=\"urn:game\">"),
            valid.replace("<Game>", "<Game xml:lang=\"en\">"),
            valid.replace("</Game>", "<g:Other/></Game>"),
        ] {
            assert_eq!(inspect(&xml), Err(ConfigError::UnsupportedXml));
        }
    }

    #[test]
    fn xml_declaration_is_optional_initial_ordered_and_utf8_only() {
        let valid = game(&["AoE3DE.exe"]);
        for declaration in [
            "",
            "<?xml version='1.0'?>",
            "<?xml version='1.0' standalone='no'?>",
            "\u{feff}<?xml version='1.0' encoding='utf-8'?>",
            "\u{feff}",
        ] {
            assert_eq!(
                inspect(&format!("{declaration}{valid}")).unwrap(),
                ["aoe3de.exe"]
            );
        }
        for declaration in [
            "<?xml version='1.0' encoding='UTF-16'?>",
            "<?xml version='1.0' encoding='windows-1252'?>",
            "<?xml version='1.0' encoding='UTF8'?>",
        ] {
            assert_eq!(
                inspect(&format!("{declaration}{valid}")),
                Err(ConfigError::UnsupportedEncoding)
            );
        }
        for declaration in [
            " <?xml version='1.0'?>",
            "<!-- first --><?xml version='1.0'?>",
            "<?xml?>",
            "<?XML version='1.0'?>",
            "<?xml version='1.1'?>",
            "<?xml encoding='UTF-8' version='1.0'?>",
            "<?xml version='1.0' standalone='yes' encoding='UTF-8'?>",
            "<?xml version='1.0' standalone='maybe'?>",
            "<?xml version='1.0' extra='ignored'?>",
            "<?xml version='1.0' version='1.0'?>",
            "<?xml version='1&#46;0'?>",
        ] {
            assert!(inspect(&format!("{declaration}{valid}")).is_err());
        }
    }

    #[test]
    fn invalid_utf8_and_xml_characters_are_rejected() {
        let root = empty_fixture();
        for bytes in [&b"\xff\xfe<\0G\0a\0m\0e\0>\0"[..], &b"\xff"[..]] {
            write_config(root.path(), "MicrosoftGame.config", bytes);
            assert_eq!(
                declarations(root.path()),
                Err(ConfigError::UnsupportedEncoding)
            );
        }
        for invalid in ['\0', '\u{1}', '\u{b}', '\u{fffe}', '\u{ffff}'] {
            assert_eq!(
                inspect(&game(&["Game.exe"]).replace("</Game>", &format!("{invalid}</Game>"))),
                Err(ConfigError::MalformedXml)
            );
        }
    }

    #[test]
    fn safe_references_are_decoded_once_before_path_validation() {
        assert_eq!(
            inspect(&game(&["AoE&#51;DE.exe", "Bin\\Game&#x32;.exe"])).unwrap(),
            ["aoe3de.exe", "bin\\game2.exe"]
        );
        assert_eq!(
            inspect(&game(&["Game&amp;amp;.exe"])).unwrap(),
            ["game&amp;.exe"]
        );
        assert_eq!(
            decode("&lt;&gt;&amp;&apos;&quot;&#65;&#x42;", true).unwrap(),
            "<>&'\"AB"
        );
        assert_eq!(
            decode("a\r\nb\rc\nd\te&#9;f", true).unwrap(),
            "a b c d e\tf"
        );
        for invalid in [
            "&custom;",
            "&AMP;",
            "&unfinished",
            "&#;",
            "&#x;",
            "&#X41;",
            "&#-1;",
            "&#+65;",
            "&#x+41;",
            "&#0;",
            "&#xD800;",
            "&#x110000;",
            "&#999999999999999999999;",
        ] {
            assert!(inspect(&game(&[&format!("Game{invalid}.exe")])).is_err());
        }
        assert_eq!(
            inspect(&game(&["&#46;&#46;\\Outside.exe"])),
            Err(ConfigError::InvalidDeclaration)
        );
        assert_eq!(
            inspect(&game(&["Game&#58;stream.exe"])),
            Err(ConfigError::InvalidDeclaration)
        );
        assert!(
            inspect(&game(&["Game.exe"]).replace("</Game>", "<Other>&custom;</Other></Game>"))
                .is_err()
        );
    }

    #[test]
    fn unsafe_names_are_not_declarations() {
        for name in [
            "",
            "..\\Outside.exe",
            "../Outside.exe",
            "Bin\\..\\Outside.exe",
            "C:\\Game.exe",
            "C:Game.exe",
            "\\Game.exe",
            "\\\\server\\share\\Game.exe",
            "\\\\?\\C:\\Game.exe",
            "Game.exe:stream",
            "Game.dll",
            "Game.exe.",
            "NUL.exe",
            "Bin\\CON.exe",
        ] {
            assert_eq!(
                inspect(&game(&[name])),
                Err(ConfigError::InvalidDeclaration),
                "{name:?}"
            );
        }
        assert_eq!(
            inspect("<Game><ExecutableList><Executable/></ExecutableList></Game>"),
            Err(ConfigError::InvalidDeclaration)
        );
        assert!(inspect(
            r#"<Game><ExecutableList><Executable Name="A.exe" Name="B.exe"/></ExecutableList></Game>"#
        )
        .is_err());
    }

    #[test]
    fn duplicate_normalized_declarations_are_ambiguous() {
        for names in [
            ["Game.exe", "Game.exe"],
            ["Game.exe", "GAME.EXE"],
            ["Bin\\Game.exe", "bin/game.EXE"],
            ["Game.exe", "G&#97;me.exe"],
        ] {
            assert_eq!(
                inspect(&game(&names)),
                Err(ConfigError::DuplicateDeclaration)
            );
        }
        let xml = "<Game><ExecutableList><Executable Name='Game.exe'/></ExecutableList>\
            <ExecutableList><Executable Name='Other.exe'/></ExecutableList></Game>";
        assert_eq!(inspect(xml).unwrap(), ["game.exe", "other.exe"]);
        assert_eq!(
            inspect(&xml.replace("Other.exe", "GAME.EXE")),
            Err(ConfigError::DuplicateDeclaration)
        );
    }

    #[test]
    fn depth_node_attribute_and_declaration_limits_are_bounded() {
        let valid = game(&["Game.exe"]);
        let nesting = format!(
            "{}{}",
            "<Node>".repeat(MAX_DEPTH - 1),
            "</Node>".repeat(MAX_DEPTH - 1)
        );
        assert!(inspect(&valid.replace("</Game>", &format!("{nesting}</Game>"))).is_ok());
        let nesting = format!(
            "{}{}",
            "<Node>".repeat(MAX_DEPTH),
            "</Node>".repeat(MAX_DEPTH)
        );
        assert_eq!(
            inspect(&valid.replace("</Game>", &format!("{nesting}</Game>"))),
            Err(ConfigError::TooDeep)
        );
        // Game, ExecutableList, and Executable account for the first three nodes.
        for node in ["<Node/>", "<!---->"] {
            let nodes = node.repeat(MAX_NODES - 3);
            assert!(inspect(&valid.replace("</Game>", &format!("{nodes}</Game>"))).is_ok());
            assert_eq!(
                inspect(&valid.replace("</Game>", &format!("{nodes}{node}</Game>"))),
                Err(ConfigError::TooManyNodes)
            );
            assert_eq!(
                inspect(&valid.replace("</Game>", &format!("{nodes}\n</Game>"))),
                Err(ConfigError::TooManyNodes)
            );
        }
        let attributes = (0..MAX_ATTRIBUTES)
            .map(|index| format!(" a{index}='value'"))
            .collect::<String>();
        assert!(inspect(&valid.replace("<Game>", &format!("<Game{attributes}>"))).is_ok());
        assert_eq!(
            inspect(&valid.replace("<Game>", &format!("<Game{attributes} extra='value'>"))),
            Err(ConfigError::TooManyAttributes)
        );
        let names = (0..MAX_DECLARATIONS + 1)
            .map(|index| format!("Game{index}.exe"))
            .collect::<Vec<_>>();
        let names = names.iter().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(
            inspect(&game(&names[..MAX_DECLARATIONS])).unwrap().len(),
            MAX_DECLARATIONS
        );
        assert_eq!(
            inspect(&game(&names)),
            Err(ConfigError::TooManyDeclarations)
        );
    }

    #[test]
    fn an_expired_budget_fails_without_filesystem_work() {
        let expired = Instant::now()
            .checked_sub(TIME_BUDGET + Duration::from_millis(1))
            .unwrap();
        assert_eq!(
            parse_document(game(&["Game.exe"]).as_bytes(), "", expired),
            Err(ConfigError::TimedOut)
        );
    }

    #[test]
    fn relative_and_remote_root_syntax_is_rejected_before_probing() {
        assert_eq!(
            declarations(Path::new("relative-install-root")),
            Err(ConfigError::UnsafeConfigPath)
        );
        #[cfg(windows)]
        for root in [
            r"\\server\share\install",
            r"\\?\UNC\server\share\install",
            r"\\?\C:\install",
            r"\\.\C:\install",
        ] {
            assert_eq!(
                declarations(Path::new(root)),
                Err(ConfigError::UnsafeConfigPath)
            );
        }
    }

    #[test]
    fn invalid_root_metadata_fails_before_config_probing() {
        let root = empty_fixture();
        assert_eq!(
            declarations(&root.path().join("missing-root")),
            Err(ConfigError::UnsafeConfigPath)
        );
        let not_a_directory = root.path().join("not-a-directory");
        fs::write(&not_a_directory, b"not an installation directory").unwrap();
        assert_eq!(
            declarations(&not_a_directory),
            Err(ConfigError::UnsafeConfigPath)
        );
    }
}
