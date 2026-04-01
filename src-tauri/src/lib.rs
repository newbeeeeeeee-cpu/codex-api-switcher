use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

const CODEX_DIR_NAME: &str = ".codex";
const AUTH_FILE_NAME: &str = "auth.json";
const CONFIG_FILE_NAME: &str = "config.toml";
const PROFILE_STORE_DIR_NAME: &str = "account-switcher";
const PROFILE_STORE_FILE_NAME: &str = "profiles.json";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CurrentConfig {
    api_key: String,
    base_url: String,
    auth_path: String,
    config_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AccountProfile {
    id: String,
    name: String,
    api_key: String,
    base_url: String,
    updated_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    current: CurrentConfig,
    profiles: Vec<AccountProfile>,
    profile_store_path: String,
    codex_dir_path: String,
    platform_label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveProfileResult {
    profiles: Vec<AccountProfile>,
    saved_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveProfileInput {
    id: Option<String>,
    name: String,
    api_key: String,
    base_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyProfileInput {
    api_key: String,
    base_url: String,
}

#[tauri::command]
fn load_snapshot() -> Result<AppSnapshot, String> {
    Ok(AppSnapshot {
        current: read_current_config()?,
        profiles: read_profiles()?,
        profile_store_path: profile_store_path()?.display().to_string(),
        codex_dir_path: codex_dir()?.display().to_string(),
        platform_label: platform_label().to_string(),
    })
}

#[tauri::command]
fn save_profile(input: SaveProfileInput) -> Result<SaveProfileResult, String> {
    let name = input.name.trim();
    let api_key = input.api_key.trim();
    let base_url = input.base_url.trim();

    if name.is_empty() {
        return Err("账号名称不能为空。".into());
    }

    if api_key.is_empty() || base_url.is_empty() {
        return Err("API Key 和 base_url 不能为空。".into());
    }

    let mut profiles = read_profiles()?;
    let now = current_timestamp();

    let saved_id = if let Some(id) = input.id.filter(|value| !value.trim().is_empty()) {
        if let Some(profile) = profiles.iter_mut().find(|profile| profile.id == id) {
            profile.name = name.to_string();
            profile.api_key = api_key.to_string();
            profile.base_url = base_url.to_string();
            profile.updated_at = now;
            id
        } else {
            let new_id = generate_profile_id();
            profiles.push(AccountProfile {
                id: new_id.clone(),
                name: name.to_string(),
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                updated_at: now,
            });
            new_id
        }
    } else {
        let new_id = generate_profile_id();
        profiles.push(AccountProfile {
            id: new_id.clone(),
            name: name.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            updated_at: now,
        });
        new_id
    };

    sort_profiles(&mut profiles);
    write_profiles(&profiles)?;

    Ok(SaveProfileResult { profiles, saved_id })
}

#[tauri::command]
fn delete_profile(id: String) -> Result<Vec<AccountProfile>, String> {
    let mut profiles = read_profiles()?;
    let original_len = profiles.len();
    profiles.retain(|profile| profile.id != id);

    if profiles.len() == original_len {
        return Err("找不到要删除的账号。".into());
    }

    sort_profiles(&mut profiles);
    write_profiles(&profiles)?;
    Ok(profiles)
}

#[tauri::command]
fn apply_profile(input: ApplyProfileInput) -> Result<CurrentConfig, String> {
    let api_key = input.api_key.trim();
    let base_url = input.base_url.trim();

    if api_key.is_empty() || base_url.is_empty() {
        return Err("API Key 和 base_url 不能为空。".into());
    }

    let auth_path = auth_path()?;
    let config_path = config_path()?;

    let auth_content =
        fs::read_to_string(&auth_path).map_err(|error| format!("读取 auth.json 失败: {error}"))?;
    let config_content = fs::read_to_string(&config_path)
        .map_err(|error| format!("读取 config.toml 失败: {error}"))?;

    let updated_auth = replace_auth_api_key(&auth_content, api_key)?;
    let updated_config = replace_openai_base_url(&config_content, base_url)?;

    fs::write(&auth_path, updated_auth).map_err(|error| format!("写入 auth.json 失败: {error}"))?;
    fs::write(&config_path, updated_config)
        .map_err(|error| format!("写入 config.toml 失败: {error}"))?;

    read_current_config()
}

fn read_current_config() -> Result<CurrentConfig, String> {
    let auth_path = auth_path()?;
    let config_path = config_path()?;

    let auth_content =
        fs::read_to_string(&auth_path).map_err(|error| format!("读取 auth.json 失败: {error}"))?;
    let config_content = fs::read_to_string(&config_path)
        .map_err(|error| format!("读取 config.toml 失败: {error}"))?;

    let json: Value = serde_json::from_str(&auth_content)
        .map_err(|error| format!("解析 auth.json 失败: {error}"))?;

    let api_key = json
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .ok_or_else(|| "auth.json 里找不到 OPENAI_API_KEY。".to_string())?
        .to_string();

    let base_url = read_openai_base_url(&config_content)?;

    Ok(CurrentConfig {
        api_key,
        base_url,
        auth_path: auth_path.display().to_string(),
        config_path: config_path.display().to_string(),
    })
}

fn read_profiles() -> Result<Vec<AccountProfile>, String> {
    let path = profile_store_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents =
        fs::read_to_string(&path).map_err(|error| format!("读取账号存储文件失败: {error}"))?;

    let mut profiles: Vec<AccountProfile> = serde_json::from_str(&contents)
        .map_err(|error| format!("解析账号存储文件失败: {error}"))?;

    sort_profiles(&mut profiles);
    Ok(profiles)
}

fn write_profiles(profiles: &[AccountProfile]) -> Result<(), String> {
    let path = profile_store_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "账号存储目录无效。".to_string())?;

    fs::create_dir_all(parent).map_err(|error| format!("创建账号存储目录失败: {error}"))?;

    let json = serde_json::to_string_pretty(profiles)
        .map_err(|error| format!("序列化账号列表失败: {error}"))?;

    fs::write(path, json).map_err(|error| format!("写入账号存储文件失败: {error}"))
}

fn auth_path() -> Result<PathBuf, String> {
    Ok(codex_dir()?.join(AUTH_FILE_NAME))
}

fn config_path() -> Result<PathBuf, String> {
    Ok(codex_dir()?.join(CONFIG_FILE_NAME))
}

fn profile_store_path() -> Result<PathBuf, String> {
    Ok(codex_dir()?
        .join(PROFILE_STORE_DIR_NAME)
        .join(PROFILE_STORE_FILE_NAME))
}

fn codex_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(CODEX_DIR_NAME))
}

fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir()
        .or_else(home_dir_from_env)
        .ok_or_else(|| "无法解析当前用户的 home 目录。".to_string())
}

fn home_dir_from_env() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| {
            let home_drive = env::var_os("HOMEDRIVE")?;
            let home_path = env::var_os("HOMEPATH")?;
            let mut path = PathBuf::from(home_drive);
            path.push(home_path);
            Some(path)
        })
    }

    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn platform_label() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unknown"
    }
}

fn sort_profiles(profiles: &mut [AccountProfile]) {
    profiles.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn generate_profile_id() -> String {
    format!("profile-{}", current_timestamp())
}

fn read_openai_base_url(content: &str) -> Result<String, String> {
    let (_, section) = openai_section(content)?;
    let captures = base_url_read_regex()
        .captures(section)
        .ok_or_else(|| "在 [model_providers.OpenAI] 里找不到 base_url。".to_string())?;

    Ok(captures
        .get(1)
        .map(|value| value.as_str().to_string())
        .unwrap_or_default())
}

fn replace_auth_api_key(content: &str, api_key: &str) -> Result<String, String> {
    let json_value =
        serde_json::to_string(api_key).map_err(|error| format!("序列化 API Key 失败: {error}"))?;

    let updated = auth_key_regex().replace(content, |captures: &Captures| {
        format!("{}{}", &captures[1], json_value)
    });

    if updated == content {
        return Err("auth.json 里找不到 OPENAI_API_KEY。".into());
    }

    Ok(updated.into_owned())
}

fn replace_openai_base_url(content: &str, base_url: &str) -> Result<String, String> {
    let ((start, end), section) = openai_section(content)?;
    let quoted_base_url = toml_basic_string(base_url);

    let updated_section = base_url_write_regex().replace(section, |captures: &Captures| {
        format!("{}{}{}", &captures[1], quoted_base_url, &captures[2])
    });

    if updated_section == section {
        return Err("在 [model_providers.OpenAI] 里找不到 base_url。".into());
    }

    let mut updated = String::with_capacity(content.len() + quoted_base_url.len());
    updated.push_str(&content[..start]);
    updated.push_str(&updated_section);
    updated.push_str(&content[end..]);
    Ok(updated)
}

fn openai_section(content: &str) -> Result<((usize, usize), &str), String> {
    let header = openai_section_regex()
        .find(content)
        .ok_or_else(|| "config.toml 里找不到 [model_providers.OpenAI]。".to_string())?;

    let after_header = header.end();
    let rest = &content[after_header..];
    let next_section = next_section_regex()
        .find(rest)
        .map(|section| after_header + section.start())
        .unwrap_or_else(|| content.len());

    Ok((
        (header.start(), next_section),
        &content[header.start()..next_section],
    ))
}

fn toml_basic_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');

    for character in value.chars() {
        match character {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            control if control.is_control() => {
                result.push_str(&format!("\\u{:04X}", control as u32));
            }
            regular => result.push(regular),
        }
    }

    result.push('"');
    result
}

fn auth_key_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"("OPENAI_API_KEY"\s*:\s*)("(?:[^"\\]|\\.)*")"#)
            .expect("OPENAI_API_KEY regex must compile")
    })
}

fn openai_section_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?m)^\[model_providers\.OpenAI\]\s*$"#)
            .expect("OpenAI section regex must compile")
    })
}

fn next_section_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?m)^\["#).expect("next section regex must compile"))
}

fn base_url_read_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?m)^\s*base_url\s*=\s*"((?:[^"\\]|\\.)*)"\s*(?:#.*)?$"#)
            .expect("base_url read regex must compile")
    })
}

fn base_url_write_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?m)^(\s*base_url\s*=\s*)"(?:[^"\\]|\\.)*"(\s*(?:#.*)?)$"#)
            .expect("base_url write regex must compile")
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_snapshot,
            save_profile,
            delete_profile,
            apply_profile
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{read_openai_base_url, replace_auth_api_key, replace_openai_base_url};

    #[test]
    fn replaces_only_openai_base_url_line() {
        let original = r#"model_provider = "OpenAI"
model = "gpt-5.4"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "http://localhost:8080"
wire_api = "responses"
requires_openai_auth = true

[projects."/Users/test"]
trust_level = "trusted"
"#;

        let updated = replace_openai_base_url(original, "https://example.com/v1")
            .expect("should replace base_url");

        assert!(updated.contains(r#"base_url = "https://example.com/v1""#));
        assert!(updated.contains(r#"wire_api = "responses""#));
        assert!(updated.contains(r#"[projects."/Users/test"]"#));
    }

    #[test]
    fn replaces_only_auth_api_key_value() {
        let original = r#"{
  "OPENAI_API_KEY": "sk-old",
  "ANOTHER_KEY": "keep-me"
}"#;

        let updated =
            replace_auth_api_key(original, "sk-new").expect("should replace OPENAI_API_KEY");

        assert!(updated.contains(r#""OPENAI_API_KEY": "sk-new""#));
        assert!(updated.contains(r#""ANOTHER_KEY": "keep-me""#));
    }

    #[test]
    fn reads_openai_base_url_from_section() {
        let config = r#"[model_providers.OpenAI]
name = "OpenAI"
base_url = "http://localhost:8080"
wire_api = "responses"
"#;

        let base_url = read_openai_base_url(config).expect("should read base_url");
        assert_eq!(base_url, "http://localhost:8080");
    }

    #[test]
    fn replaces_base_url_in_crlf_config() {
        let original = "[model_providers.OpenAI]\r\nname = \"OpenAI\"\r\nbase_url = \"http://localhost:8080\"\r\nwire_api = \"responses\"\r\n";

        let updated = replace_openai_base_url(original, "https://example.com/v1")
            .expect("should replace base_url in CRLF config");

        assert!(updated.contains("base_url = \"https://example.com/v1\"\r\n"));
        assert!(updated.contains("\r\nwire_api = \"responses\"\r\n"));
    }
}
