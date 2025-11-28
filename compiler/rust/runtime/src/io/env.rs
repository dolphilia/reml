use std::env;

/// Core.IO から参照できるタイムゾーン／ロケールの環境情報。
#[derive(Debug, Clone)]
pub struct TimeEnvSnapshot {
    timezone_env: Option<String>,
    locale_env: Option<String>,
    platform: &'static str,
}

impl TimeEnvSnapshot {
    pub fn timezone_env(&self) -> Option<&str> {
        self.timezone_env.as_deref()
    }

    pub fn locale_env(&self) -> Option<&str> {
        self.locale_env.as_deref()
    }

    pub fn platform(&self) -> &'static str {
        self.platform
    }
}

/// 環境変数 (`TZ`, `LC_TIME`, `LANG`) を読み取り、TimeError や Diagnostics へ渡すスナップショットを返す。
pub fn time_env_snapshot() -> TimeEnvSnapshot {
    let timezone_env = read_var("TZ");
    let locale_env = read_var("LC_TIME").or_else(|| read_var("LANG"));
    TimeEnvSnapshot {
        timezone_env,
        locale_env,
        platform: std::env::consts::OS,
    }
}

fn read_var(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(_) => None,
    }
}
