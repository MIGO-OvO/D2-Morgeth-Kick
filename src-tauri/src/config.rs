use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::Shortcut;

use crate::input;

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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeyConfig {
    pub start: String,
    pub stop: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            start: "F8".into(),
            stop: "F10".into(),
        }
    }
}

impl HotkeyConfig {
    pub fn shortcuts(&self) -> Result<[Shortcut; 2], String> {
        let start = self
            .start
            .parse::<Shortcut>()
            .map_err(|error| format!("启动热键无效：{error}"))?;
        let stop = self
            .stop
            .parse::<Shortcut>()
            .map_err(|error| format!("停止热键无效：{error}"))?;
        if start == stop {
            return Err("启动热键和停止热键不能相同".into());
        }
        Ok([start, stop])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct GameKeyConfig {
    pub sprint: String,
    pub jump: String,
    pub interact: String,
    pub weapon_slot_2: String,
    pub melee: String,
    pub ascension: String,
    pub super_ability: String,
    pub finisher: String,
}

impl Default for GameKeyConfig {
    fn default() -> Self {
        Self {
            sprint: "ShiftLeft".into(),
            jump: "Space".into(),
            interact: "KeyE".into(),
            weapon_slot_2: "Digit2".into(),
            melee: "KeyC".into(),
            ascension: "KeyX".into(),
            super_ability: "KeyF".into(),
            finisher: "KeyG".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedGameKeys {
    pub sprint: u16,
    pub jump: u16,
    pub interact: u16,
    pub weapon_slot_2: u16,
    pub melee: u16,
    pub ascension: u16,
    pub super_ability: u16,
    pub finisher: u16,
}

impl AppliedGameKeys {
    pub fn all(self) -> [u16; 8] {
        [
            self.sprint,
            self.jump,
            self.interact,
            self.weapon_slot_2,
            self.melee,
            self.ascension,
            self.super_ability,
            self.finisher,
        ]
    }
}

impl GameKeyConfig {
    pub fn applied(&self) -> Result<AppliedGameKeys, String> {
        let resolve = |name: &str, binding: &str| {
            input::scan_code(binding).ok_or_else(|| format!("{name}按键不受支持：{binding}"))
        };
        Ok(AppliedGameKeys {
            sprint: resolve("切换冲刺", &self.sprint)?,
            jump: resolve("跳跃", &self.jump)?,
            interact: resolve("插旗/交互", &self.interact)?,
            weapon_slot_2: resolve("切换到 2 号位武器", &self.weapon_slot_2)?,
            melee: resolve("近战", &self.melee)?,
            ascension: resolve("飞升", &self.ascension)?,
            super_ability: resolve("超能", &self.super_ability)?,
            finisher: resolve("终结技", &self.finisher)?,
        })
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
    pub field_of_view: f64,
    pub reference_look_sensitivity: f64,
    pub reference_ads_modifier: f64,
    pub reference_field_of_view: f64,
    pub first_ads_base: [i32; 2],
    pub void_arrow_base: [i32; 2],
    pub void_arrow_trim: [i32; 2],
    pub sprint_base: [i32; 2],
    pub sprint_trim: [i32; 2],
    pub timings: TimingConfig,
    pub hotkeys: HotkeyConfig,
    pub game_keys: GameKeyConfig,
    pub overlay_visible: bool,
    pub overlay_opacity: f64,
    pub usage_guide_seen: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            resolution_mode: ResolutionMode::Auto,
            manual_width: 1920,
            manual_height: 1080,
            look_sensitivity: 15.0,
            ads_modifier: 1.0,
            field_of_view: 100.0,
            reference_look_sensitivity: 15.0,
            reference_ads_modifier: 1.0,
            reference_field_of_view: 100.0,
            first_ads_base: [-2600, 50],
            void_arrow_base: [-300, 81],
            void_arrow_trim: [0, 0],
            sprint_base: [280, 0],
            sprint_trim: [0, 0],
            timings: TimingConfig::default(),
            hotkeys: HotkeyConfig::default(),
            game_keys: GameKeyConfig::default(),
            overlay_visible: true,
            overlay_opacity: 0.88,
            usage_guide_seen: false,
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
            ("参考视角灵敏度", self.reference_look_sensitivity),
        ] {
            if !value.is_finite() || !(1.0..=100.0).contains(&value) {
                return Err(format!("{name}必须在 1 到 100 之间"));
            }
        }
        for (name, value) in [
            ("瞄准灵敏度", self.ads_modifier),
            ("参考瞄准灵敏度", self.reference_ads_modifier),
        ] {
            if !value.is_finite() || !(0.5..=1.5).contains(&value) {
                return Err(format!("{name}必须在 0.5 到 1.5 之间"));
            }
        }
        for (name, value) in [
            ("视野范围", self.field_of_view),
            ("参考视野范围", self.reference_field_of_view),
        ] {
            if !value.is_finite() || !(55.0..=105.0).contains(&value) {
                return Err(format!("{name}必须在 55 到 105 之间"));
            }
        }
        for (name, value) in [
            ("飞升后等待", self.timings.ascension_wait),
            ("近战额外等待", self.timings.melee_extra_wait),
            ("ADS 至超能", self.timings.ads_to_super_wait),
            ("超能后等待", self.timings.super_wait),
            ("冲刺侧移时间", self.timings.sprint_a_time),
            ("冲刺至终结", self.timings.sprint_to_finisher),
        ] {
            if !value.is_finite() || !(0.0..=60.0).contains(&value) {
                return Err(format!("{name} 必须在 0 到 60 秒之间"));
            }
        }
        if !self.overlay_opacity.is_finite() || !(0.3..=1.0).contains(&self.overlay_opacity) {
            return Err("悬浮窗透明度必须在 30% 到 100% 之间".into());
        }
        self.hotkeys.shortcuts()?;
        self.game_keys.applied()?;
        Ok(())
    }

    pub fn ads_scale(&self) -> f64 {
        (self.reference_look_sensitivity * self.reference_ads_modifier)
            / (self.look_sensitivity * self.ads_modifier)
            * self.fov_scale()
    }

    pub fn look_scale(&self) -> f64 {
        self.reference_look_sensitivity / self.look_sensitivity * self.fov_scale()
    }

    pub fn fov_scale(&self) -> f64 {
        let current_half_angle = (self.field_of_view.to_radians() / 2.0).tan();
        let reference_half_angle = (self.reference_field_of_view.to_radians() / 2.0).tan();
        current_half_angle / reference_half_angle
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
        let config = AppConfig {
            look_sensitivity: 10.0,
            ads_modifier: 1.5,
            void_arrow_trim: [5, -3],
            ..AppConfig::default()
        };
        assert_eq!(config.first_ads_offset(), [-2600, 50]);
        assert_eq!(config.void_arrow_offset(), [-445, 119]);
    }

    #[test]
    fn fov_uses_tangent_projection_scaling() {
        let config = AppConfig {
            field_of_view: 90.0,
            ..AppConfig::default()
        };
        let expected = (45_f64.to_radians()).tan() / (50_f64.to_radians()).tan();
        assert!((config.fov_scale() - expected).abs() < 1e-10);
        assert_eq!(config.first_ads_offset(), [-2182, 42]);
        assert_eq!(config.void_arrow_offset(), [-252, 68]);
        assert_eq!(config.sprint_offset(), [235, 0]);
    }

    #[test]
    fn fov_stays_within_destiny_slider_range() {
        let too_narrow = AppConfig {
            field_of_view: 54.0,
            ..AppConfig::default()
        };
        let too_wide_reference = AppConfig {
            reference_field_of_view: 106.0,
            ..AppConfig::default()
        };
        assert_eq!(
            too_narrow.validate().unwrap_err(),
            "视野范围必须在 55 到 105 之间"
        );
        assert_eq!(
            too_wide_reference.validate().unwrap_err(),
            "参考视野范围必须在 55 到 105 之间"
        );
    }

    #[test]
    fn legacy_settings_without_fov_keep_the_reference_default() {
        let config: AppConfig = serde_json::from_str(r#"{"lookSensitivity": 10}"#).unwrap();
        assert_eq!(config.look_sensitivity, 10.0);
        assert_eq!(config.field_of_view, 100.0);
        assert_eq!(config.reference_field_of_view, 100.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn sensitivity_stays_within_destiny_slider_ranges() {
        let invalid_look = AppConfig {
            look_sensitivity: 0.0,
            ..AppConfig::default()
        };
        let invalid_aim = AppConfig {
            ads_modifier: 1.6,
            ..AppConfig::default()
        };
        assert_eq!(
            invalid_look.validate().unwrap_err(),
            "视角灵敏度必须在 1 到 100 之间"
        );
        assert_eq!(
            invalid_aim.validate().unwrap_err(),
            "瞄准灵敏度必须在 0.5 到 1.5 之间"
        );
    }

    #[test]
    fn default_shortcuts_and_game_keys_are_valid() {
        let config = AppConfig::default();
        assert!(config.hotkeys.shortcuts().is_ok());
        assert_eq!(
            config.game_keys.applied().unwrap().all(),
            [0x2a, 0x39, 0x12, 0x03, 0x2e, 0x2d, 0x21, 0x22]
        );
    }

    #[test]
    fn duplicate_global_shortcuts_are_rejected() {
        let config = AppConfig {
            hotkeys: HotkeyConfig {
                start: "F8".into(),
                stop: "F8".into(),
            },
            ..AppConfig::default()
        };
        assert_eq!(config.validate().unwrap_err(), "启动热键和停止热键不能相同");
    }

    #[test]
    fn overlay_opacity_stays_within_user_control_range() {
        let too_transparent = AppConfig {
            overlay_opacity: 0.29,
            ..AppConfig::default()
        };
        let fully_visible = AppConfig {
            overlay_opacity: 1.0,
            ..AppConfig::default()
        };
        assert_eq!(
            too_transparent.validate().unwrap_err(),
            "悬浮窗透明度必须在 30% 到 100% 之间"
        );
        assert!(fully_visible.validate().is_ok());
    }
}
