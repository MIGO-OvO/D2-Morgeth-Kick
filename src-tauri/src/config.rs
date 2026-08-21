use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TimingConfig {
    pub ascension_wait: f64,
    pub melee_extra_wait: f64,
    pub ads_to_super_wait: f64,
    pub super_wait: f64,
    pub sprint_a_time: f64,
    pub sprint_to_finisher: f64,
    pub finisher_wait: f64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            ascension_wait: 1.6,
            melee_extra_wait: 0.5,
            ads_to_super_wait: 2.5,
            super_wait: 1.8,
            sprint_a_time: 0.1,
            sprint_to_finisher: 0.0,
            finisher_wait: 3.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub resolution_mode: ResolutionMode,
    pub manual_width: u32,
    pub manual_height: u32,
    pub look_sensitivity: f64,
    pub ads_modifier: f64,
    pub reference_look_sensitivity: f64,
    pub reference_ads_modifier: f64,
    pub first_ads_base: [i32; 2],
    pub void_arrow_base: [i32; 2],
    pub void_arrow_trim: [i32; 2],
    pub sprint_base: [i32; 2],
    pub sprint_trim: [i32; 2],
    pub timings: TimingConfig,
    pub overlay_visible: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            resolution_mode: ResolutionMode::Auto,
            manual_width: 1920,
            manual_height: 1080,
            look_sensitivity: 15.0,
            ads_modifier: 1.0,
            reference_look_sensitivity: 15.0,
            reference_ads_modifier: 1.0,
            first_ads_base: [-2600, 50],
            void_arrow_base: [-300, 81],
            void_arrow_trim: [0, 0],
            sprint_base: [280, 0],
            sprint_trim: [0, 0],
            timings: TimingConfig::default(),
            overlay_visible: true,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=7680).contains(&self.manual_width) || !(1..=4320).contains(&self.manual_height) {
            return Err("手动分辨率超出有效范围".into());
        }
        for (name, value) in [
            ("视角灵敏度", self.look_sensitivity),
            ("ADS 修正", self.ads_modifier),
            ("参考视角灵敏度", self.reference_look_sensitivity),
            ("参考 ADS 修正", self.reference_ads_modifier),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{name} 必须大于 0"));
            }
        }
        for (name, value) in [
            ("飞升后等待", self.timings.ascension_wait),
            ("近战额外等待", self.timings.melee_extra_wait),
            ("ADS 至超能", self.timings.ads_to_super_wait),
            ("超能后等待", self.timings.super_wait),
            ("冲刺侧移时间", self.timings.sprint_a_time),
            ("冲刺至终结", self.timings.sprint_to_finisher),
            ("终结后等待", self.timings.finisher_wait),
        ] {
            if !value.is_finite() || !(0.0..=60.0).contains(&value) {
                return Err(format!("{name} 必须在 0 到 60 秒之间"));
            }
        }
        Ok(())
    }

    pub fn ads_scale(&self) -> f64 {
        (self.reference_look_sensitivity * self.reference_ads_modifier)
            / (self.look_sensitivity * self.ads_modifier)
    }

    pub fn look_scale(&self) -> f64 {
        self.reference_look_sensitivity / self.look_sensitivity
    }

    pub fn first_ads_offset(&self) -> [i32; 2] {
        scale_and_trim(self.first_ads_base, self.ads_scale(), [0, 0])
    }

    pub fn void_arrow_offset(&self) -> [i32; 2] {
        scale_and_trim(
            self.void_arrow_base,
            self.look_scale(),
            self.void_arrow_trim,
        )
    }

    pub fn sprint_offset(&self) -> [i32; 2] {
        scale_and_trim(self.sprint_base, self.look_scale(), self.sprint_trim)
    }
}

fn scale_and_trim(base: [i32; 2], scale: f64, trim: [i32; 2]) -> [i32; 2] {
    [
        (f64::from(base[0]) * scale).round() as i32 + trim[0],
        (f64::from(base[1]) * scale).round() as i32 + trim[1],
    ]
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("settings.json"))
        .map_err(|error| format!("无法确定设置目录：{error}"))
}

pub fn load(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|error| format!("无法读取设置：{error}"))?;
    let config: AppConfig =
        serde_json::from_str(&raw).map_err(|error| format!("设置文件格式无效：{error}"))?;
    config.validate()?;
    Ok(config)
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    config.validate()?;
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建设置目录：{error}"))?;
    }
    let raw =
        serde_json::to_string_pretty(config).map_err(|error| format!("无法序列化设置：{error}"))?;
    fs::write(path, raw).map_err(|error| format!("无法保存设置：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_offsets_match_python_reference() {
        let config = AppConfig::default();
        assert_eq!(config.first_ads_offset(), [-2600, 50]);
        assert_eq!(config.void_arrow_offset(), [-300, 81]);
        assert_eq!(config.sprint_offset(), [280, 0]);
    }

    #[test]
    fn sensitivity_scales_offsets_and_keeps_trim_absolute() {
        let mut config = AppConfig::default();
        config.look_sensitivity = 10.0;
        config.ads_modifier = 1.5;
        config.void_arrow_trim = [5, -3];
        assert_eq!(config.first_ads_offset(), [-2600, 50]);
        assert_eq!(config.void_arrow_offset(), [-445, 119]);
    }
}
