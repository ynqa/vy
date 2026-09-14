//! Configuration file loading, initial creation, editing, and completion keys.

use std::{
    env,
    error::Error,
    fmt,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result, anyhow, bail};
use toml_edit::{DocumentMut, Item, Table, Value, value};

use super::{Config, DEFAULT_CONFIG};

#[derive(Debug)]
pub(crate) enum ConfigLoadError {
    CheckFile {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateFile {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseFile {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckFile { path, source } => write!(
                formatter,
                "failed to check configuration file `{}`: {source}",
                path.display()
            ),
            Self::CreateDirectory { path, source } => write!(
                formatter,
                "failed to create configuration directory `{}`: {source}",
                path.display()
            ),
            Self::CreateFile { path, source } => write!(
                formatter,
                "failed to create configuration file `{}`: {source}",
                path.display()
            ),
            Self::WriteFile { path, source } => write!(
                formatter,
                "failed to write configuration file `{}`: {source}",
                path.display()
            ),
            Self::ReadFile { path, source } => write!(
                formatter,
                "failed to read configuration file `{}`: {source}",
                path.display()
            ),
            Self::ParseFile { path, source } => write!(
                formatter,
                "failed to parse configuration file `{}`: {source}",
                path.display()
            ),
        }
    }
}

impl Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CheckFile { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::CreateFile { source, .. }
            | Self::WriteFile { source, .. }
            | Self::ReadFile { source, .. } => Some(source),
            Self::ParseFile { source, .. } => Some(source),
        }
    }
}

pub(crate) struct ConfigFile {
    path: Option<PathBuf>,
    is_default_path: bool,
}

impl ConfigFile {
    /// Uses the explicit path, or resolves the platform's default configuration path.
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        let is_default_path = path.is_none();
        Self {
            path: path
                .or_else(|| dirs::config_dir().map(|path| path.join("vy").join("config.toml"))),
            is_default_path,
        }
    }

    /// Loads settings at startup, creating a missing default file first.
    /// Explicit paths must already exist. If no platform path is available,
    /// uses the embedded defaults.
    pub(crate) fn load_initial(&self) -> Result<Config, ConfigLoadError> {
        let Some(path) = self.path.as_deref() else {
            return Ok(Config::default());
        };
        if self.is_default_path {
            Self::ensure_default_config_file_exists(path)?;
        }
        Self::load_file(path)
    }

    pub(crate) fn read(&self) -> Result<String> {
        match &self.path {
            Some(path) => fs::read_to_string(path)
                .with_context(|| format!("failed to read configuration file `{}`", path.display())),
            None => Ok(DEFAULT_CONFIG.to_owned()),
        }
    }

    /// Reloads settings without creating a missing file.
    pub(crate) fn load(&self) -> Result<Config> {
        let content = self.read()?;
        Config::load_from(&content).with_context(|| self.invalid_config_message())
    }

    pub(crate) fn get(&self, key: &str) -> Result<String> {
        let content = self.read()?;
        let document = parse_document(&content, self.path.as_deref())?;
        get(&document, key)
    }

    pub(crate) fn set(&self, key: &str, raw_value: &str) -> Result<Config> {
        if !keys().iter().any(|known| known == key) {
            bail!("configuration key `{key}` was not found");
        }
        let path = self.writable_path()?;
        let content = self.read()?;
        let mut document = parse_document(&content, Some(path))?;
        let segments = key_segments(key)?;
        let value = parse_value(raw_value);
        set_in_table(document.as_table_mut(), &segments, value, key)?;

        let updated = document.to_string();
        let config = Config::load_from(&updated).with_context(|| {
            format!("invalid value `{raw_value}` for configuration key `{key}`")
        })?;
        fs::write(path, updated)
            .with_context(|| format!("failed to write configuration file `{}`", path.display()))?;
        Ok(config)
    }

    pub(crate) fn edit(&self) -> Result<()> {
        let path = self.writable_path()?;
        let editor = env::var("VISUAL")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| default_editor().to_owned());
        let mut parts =
            shlex::split(&editor).ok_or_else(|| anyhow!("invalid editor command `{editor}`"))?;
        if parts.is_empty() {
            bail!("editor command is empty");
        }

        let program = parts.remove(0);
        let status = Command::new(&program)
            .args(parts)
            .arg(path)
            .status()
            .with_context(|| format!("failed to start editor `{program}`"))?;
        if !status.success() {
            bail!("editor `{program}` exited with {status}");
        }
        Ok(())
    }

    fn load_file(path: &Path) -> Result<Config, ConfigLoadError> {
        let content = fs::read_to_string(path).map_err(|source| ConfigLoadError::ReadFile {
            path: path.to_owned(),
            source,
        })?;

        Config::load_from(&content).map_err(|source| ConfigLoadError::ParseFile {
            path: path.to_owned(),
            source,
        })
    }

    fn ensure_default_config_file_exists(path: &Path) -> Result<(), ConfigLoadError> {
        if path
            .try_exists()
            .map_err(|source| ConfigLoadError::CheckFile {
                path: path.to_owned(),
                source,
            })?
        {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigLoadError::CreateDirectory {
                path: parent.to_owned(),
                source,
            })?;
        }

        let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(()),
            Err(source) => {
                return Err(ConfigLoadError::CreateFile {
                    path: path.to_owned(),
                    source,
                });
            }
        };

        file.write_all(DEFAULT_CONFIG.as_bytes())
            .map_err(|source| ConfigLoadError::WriteFile {
                path: path.to_owned(),
                source,
            })
    }

    fn writable_path(&self) -> Result<&Path> {
        self.path.as_deref().ok_or_else(|| {
            anyhow!("the platform configuration path could not be resolved; use --config <PATH>")
        })
    }

    fn invalid_config_message(&self) -> String {
        self.path.as_ref().map_or_else(
            || "embedded configuration is invalid".to_owned(),
            |path| format!("configuration file `{}` is invalid", path.display()),
        )
    }
}

pub(crate) fn keys() -> &'static [String] {
    static KEYS: OnceLock<Vec<String>> = OnceLock::new();
    KEYS.get_or_init(|| {
        let document = DEFAULT_CONFIG
            .parse::<DocumentMut>()
            .expect("default configuration must be valid TOML");
        let mut keys = Vec::new();
        collect_keys(document.as_item(), "", &mut keys);
        keys.extend(["json.lines".to_owned(), "yaml.lines".to_owned()]);
        keys.sort();
        keys
    })
}

fn collect_keys(item: &Item, prefix: &str, keys: &mut Vec<String>) {
    let Some(table) = item.as_table_like() else {
        if !prefix.is_empty() {
            keys.push(prefix.to_owned());
        }
        return;
    };

    for (key, item) in table.iter() {
        let key = if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        };
        collect_keys(item, &key, keys);
    }
}

fn parse_document(content: &str, path: Option<&Path>) -> Result<DocumentMut> {
    content.parse::<DocumentMut>().with_context(|| match path {
        Some(path) => format!("failed to parse configuration file `{}`", path.display()),
        None => "failed to parse embedded configuration".to_owned(),
    })
}

fn key_segments(key: &str) -> Result<Vec<&str>> {
    let segments = key.split('.').collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        bail!("invalid configuration key `{key}`");
    }
    Ok(segments)
}

fn get(document: &DocumentMut, key: &str) -> Result<String> {
    let mut item = document.as_item();
    for segment in key_segments(key)? {
        item = item
            .get(segment)
            .ok_or_else(|| anyhow!("configuration key `{key}` was not found"))?;
    }

    match item.as_value() {
        Some(Value::String(value)) => Ok(value.value().to_owned()),
        Some(value) => Ok(value.to_string().trim().to_owned()),
        None => Ok(item.to_string().trim().to_owned()),
    }
}

fn parse_value(raw: &str) -> Value {
    raw.parse::<Value>()
        .unwrap_or_else(|_| Value::from(raw.to_owned()))
}

fn set_in_table(table: &mut Table, path: &[&str], mut new_value: Value, key: &str) -> Result<()> {
    if path.len() == 1 {
        if let Some(Item::Value(current_value)) = table.get(path[0]) {
            *new_value.decor_mut() = current_value.decor().clone();
        }
        table[path[0]] = value(new_value);
        return Ok(());
    }

    let item = table
        .get_mut(path[0])
        .ok_or_else(|| anyhow!("configuration key `{key}` was not found"))?;
    let child = item
        .as_table_mut()
        .ok_or_else(|| anyhow!("configuration key `{key}` does not refer to a nested value"))?;
    set_in_table(child, &path[1..], new_value, key)
}

#[cfg(windows)]
fn default_editor() -> &'static str {
    "notepad"
}

#[cfg(not(windows))]
fn default_editor() -> &'static str {
    "vi"
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_config() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vy-config-command-{}-{unique}.toml",
            std::process::id()
        ))
    }

    #[test]
    fn gets_values_and_lists_completion_keys() {
        let path = temp_config();
        fs::write(&path, DEFAULT_CONFIG).unwrap();
        let file = ConfigFile::new(Some(path.clone()));

        assert_eq!(file.get("json.indent").unwrap(), "2");
        assert_eq!(file.get("json.overflow_mode").unwrap(), "Wrap");
        assert!(file.get("json.missing").is_err());
        assert!(keys().contains(&"keybinds.view.browse.input.exit".to_owned()));
        assert!(keys().contains(&"keybinds.view.browse.open_command_editor".to_owned()));
        assert!(keys().contains(&"keybinds.view.browse.move_to_parent".to_owned()));
        assert!(!keys().contains(&"keybinds.view.browse.input.open_command_editor".to_owned()));
        assert!(keys().contains(&"json.lines".to_owned()));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_unsupported_versions_without_changing_the_file() {
        let path = temp_config();
        let content = "version = 2\n[json]\n[yaml]\n";
        fs::write(&path, content).unwrap();
        let file = ConfigFile::new(Some(path.clone()));

        assert!(matches!(
            file.load_initial(),
            Err(ConfigLoadError::ParseFile { .. })
        ));
        assert!(file.load().is_err());
        assert!(file.set("json.indent", "4").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), content);

        fs::write(&path, DEFAULT_CONFIG).unwrap();
        assert_eq!(file.get("version").unwrap(), "1");
        assert!(file.set("version", "2").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn adds_child_count_settings_to_an_existing_config() {
        let path = temp_config();
        fs::write(&path, "[json]\n[yaml]\n").unwrap();
        let file = ConfigFile::new(Some(path.clone()));
        for format in ["json", "yaml"] {
            let key = format!("{format}.show_child_count");
            assert!(keys().contains(&key));
            file.set(&key, "true").unwrap();
            assert_eq!(file.get(&key).unwrap(), "true");
            assert!(file.set(&key, "invalid").is_err());
            assert_eq!(file.get(&key).unwrap(), "true");
            file.set(&format!("{format}.child_count_style"), "fg=cyan")
                .unwrap();
        }
        let config = file.load().unwrap();
        assert!(config.json.show_child_count);
        assert!(config.yaml.show_child_count);
        let cyan = Some(promkit::core::crossterm::style::Color::Cyan);
        assert_eq!(config.json.child_count_style.foreground_color, cyan);
        assert_eq!(config.yaml.child_count_style.foreground_color, cyan);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn sets_and_validates_values_without_losing_comments() {
        let path = temp_config();
        fs::write(&path, DEFAULT_CONFIG).unwrap();
        let file = ConfigFile::new(Some(path.clone()));

        let config = file.set("json.indent", "4").unwrap();
        assert_eq!(config.json.indent, 4);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Number of spaces used for each indentation level\nindent = 4"));

        assert!(file.set("json.indent", "wide").is_err());
        assert_eq!(file.get("json.indent").unwrap(), "4");
        assert!(file.set("missing.value", "1").is_err());
        assert!(file.set("missing", "1").is_err());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn explicit_missing_config_returns_an_error_without_creating_a_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("vy-config-{}-{unique}", std::process::id()));
        let path = directory.join("config.toml");

        let file = ConfigFile::new(Some(path.clone()));
        let Err(error) = file.load_initial() else {
            panic!("missing explicit configuration must return an error");
        };

        let ConfigLoadError::ReadFile {
            path: error_path, ..
        } = error
        else {
            panic!("missing explicit configuration must be a read error");
        };
        assert_eq!(error_path, path);
        assert!(!path.exists());
        assert!(!directory.exists());
    }

    #[test]
    fn creates_missing_default_config_without_overwriting_it() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("vy-default-config-{}-{unique}", std::process::id()));
        let path = directory.join("vy").join("config.toml");

        let file = ConfigFile {
            path: Some(path.clone()),
            is_default_path: true,
        };
        file.load_initial().unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);

        let custom = "[json]\nindent = 4\n[yaml]\n";
        fs::write(&path, custom).unwrap();
        assert_eq!(file.load_initial().unwrap().json.indent, 4);
        assert_eq!(fs::read_to_string(&path).unwrap(), custom);
        assert_eq!(file.load().unwrap().json.indent, 4);

        fs::write(&path, "invalid configuration").unwrap();
        assert!(matches!(
            file.load_initial(),
            Err(ConfigLoadError::ParseFile { .. })
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid configuration");

        fs::remove_file(&path).unwrap();
        assert!(file.load().is_err());
        assert!(!path.exists());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unresolved_platform_path_uses_embedded_defaults_without_allowing_writes() {
        let file = ConfigFile {
            path: None,
            is_default_path: true,
        };

        assert_eq!(file.read().unwrap(), DEFAULT_CONFIG);
        assert_eq!(file.load_initial().unwrap().json.indent, 2);
        assert_eq!(file.load().unwrap().json.indent, 2);
        assert!(file.set("json.indent", "4").is_err());
    }
}
