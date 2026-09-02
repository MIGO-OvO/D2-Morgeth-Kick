use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::Shortcut;

use crate::input::{self, InputBinding, MouseButton};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FirstAimMode {
    Ads,
    Hipfire,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingConfig {
    pub strafe_to_flag_wait: f64,
    pub flag_to_claim_wait: f64,
    pub claim_to_weapon_wait: f64,
    pub weapon_to_move_wait: f64,
    pub position_to_aim_wait: f64,
    pub aim_to_melee_wait: f64,
    pub melee_to_super_wait: f64,
    pub super_to_sprint_wait: f64,
    pub sprint_to_finisher: f64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            strafe_to_flag_wait: 0.6,
            flag_to_claim_wait: 1.0,
            claim_to_weapon_wait: 0.2,
            weapon_to_move_wait: 0.5,
            position_to_aim_wait: 1.5,
            aim_to_melee_wait: 0.9,
            melee_to_super_wait: 0.15,
            super_to_sprint_wait: 2.0,
            sprint_to_finisher: 0.0,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TimingConfigWire {
    strafe_to_flag_wait: Option<f64>,
    flag_to_claim_wait: Option<f64>,
    claim_to_weapon_wait: Option<f64>,
    weapon_to_move_wait: Option<f64>,
    position_to_aim_wait: Option<f64>,
    aim_to_melee_wait: Option<f64>,
    melee_to_super_wait: Option<f64>,
    super_to_sprint_wait: Option<f64>,
    sprint_to_finisher: Option<f64>,
    // v0.3.6 and earlier used a mix of absolute phase anchors and direct waits.
    melee_extra_wait: Option<f64>,
    ads_to_super_wait: Option<f64>,
    super_wait: Option<f64>,
}

impl<'de> Deserialize<'de> for TimingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = TimingConfigWire::deserialize(deserializer)?;
        let defaults = Self::default();
        let legacy_melee_extra = wire.melee_extra_wait.unwrap_or(0.3);
        let legacy_super_anchor = wire.ads_to_super_wait.unwrap_or(2.5);

        Ok(Self {
            strafe_to_flag_wait: wire
                .strafe_to_flag_wait
                .unwrap_or(defaults.strafe_to_flag_wait),
            flag_to_claim_wait: wire
                .flag_to_claim_wait
                .unwrap_or(defaults.flag_to_claim_wait),
            claim_to_weapon_wait: wire
                .claim_to_weapon_wait
                .unwrap_or(defaults.claim_to_weapon_wait),
            weapon_to_move_wait: wire
                .weapon_to_move_wait
                .unwrap_or(defaults.weapon_to_move_wait),
            position_to_aim_wait: wire
                .position_to_aim_wait
                .unwrap_or(defaults.position_to_aim_wait),
            // The old melee value was an extra wait after the fixed 4.047 s phase anchor.
            aim_to_melee_wait: wire.aim_to_melee_wait.unwrap_or(0.522 + legacy_melee_extra),
            // The old super value was anchored at 2.109 s, so subtract the melee end time.
            melee_to_super_wait: wire
                .melee_to_super_wait
                .unwrap_or_else(|| (legacy_super_anchor - 2.016 - legacy_melee_extra).max(0.0)),
            super_to_sprint_wait: wire
                .super_to_sprint_wait
                .or(wire.super_wait)
                .unwrap_or(defaults.super_to_sprint_wait),
            sprint_to_finisher: wire
                .sprint_to_finisher
                .unwrap_or(defaults.sprint_to_finisher),
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseShortcut {
    pub button: MouseButton,
    pub modifiers: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyBinding {
    Keyboard(Shortcut),
    Mouse(MouseShortcut),
}

fn parse_hotkey(binding: &str, name: &str) -> Result<HotkeyBinding, String> {
    let mut parts = binding.split('+').collect::<Vec<_>>();
    let primary = parts
        .pop()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| format!("{name}热键无效：缺少主键"))?;
    let mouse_button = match primary {
        "MouseMiddle" => Some(MouseButton::Middle),
        "Mouse4" => Some(MouseButton::Back),
        "Mouse5" => Some(MouseButton::Forward),
        _ => None,
    };
    if let Some(button) = mouse_button {
        let mut modifiers = 0;
        for modifier in parts {
            modifiers |= match modifier.to_ascii_lowercase().as_str() {
                "control" | "ctrl" => input::MOD_CONTROL,
                "shift" => input::MOD_SHIFT,
                "alt" => input::MOD_ALT,
                "super" | "win" => input::MOD_SUPER,
                _ => return Err(format!("{name}热键包含不受支持的修饰键：{modifier}")),
            };
        }
        Ok(HotkeyBinding::Mouse(MouseShortcut { button, modifiers }))
    } else {
        binding
            .parse::<Shortcut>()
            .map(HotkeyBinding::Keyboard)
            .map_err(|error| format!("{name}热键无效：{error}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyRole {
    Start,
    Stop,
}

impl HotkeyConfig {
    /// 单独解析某个角色的热键（供诊断记录解析成功/失败与原始错误）。
    pub fn role_binding(&self, role: HotkeyRole) -> Result<HotkeyBinding, String> {
        match role {
            HotkeyRole::Start => parse_hotkey(&self.start, "启动"),
            HotkeyRole::Stop => parse_hotkey(&self.stop, "停止"),
        }
    }

    pub fn bindings(&self) -> Result<[HotkeyBinding; 2], String> {
        let start = self.role_binding(HotkeyRole::Start)?;
        let stop = self.role_binding(HotkeyRole::Stop)?;
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
    pub sprint: InputBinding,
    pub jump: InputBinding,
    pub interact: InputBinding,
    pub weapon_slot_2: InputBinding,
    pub melee: InputBinding,
    pub ascension: InputBinding,
    pub super_ability: InputBinding,
    pub finisher: InputBinding,
}

impl AppliedGameKeys {
    pub fn all(self) -> [InputBinding; 8] {
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
            input::binding(binding).ok_or_else(|| format!("{name}按键不受支持：{binding}"))
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
    pub first_aim_mode: FirstAimMode,
    pub first_ads_base: [i32; 2],
    pub first_hip_base: [i32; 2],
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
            first_aim_mode: FirstAimMode::Ads,
            first_ads_base: [-2600, 50],
            first_hip_base: [-1320, 30],
            void_arrow_base: [-300, 81],
            void_arrow_trim: [-50, 0],
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
            ("左移结束至插旗", self.timings.strafe_to_flag_wait),
            ("插旗结束至摸旗", self.timings.flag_to_claim_wait),
            ("摸旗结束至切武器", self.timings.claim_to_weapon_wait),
            ("切武器结束至开始移动", self.timings.weapon_to_move_wait),
            ("后退定位结束至首次转向", self.timings.position_to_aim_wait),
            ("首次转向结束至近战", self.timings.aim_to_melee_wait),
            ("近战结束至虚空箭", self.timings.melee_to_super_wait),
            ("超能结束至冲刺", self.timings.super_to_sprint_wait),
            ("冲刺转向结束至终结", self.timings.sprint_to_finisher),
        ] {
            if !value.is_finite() || !(0.0..=10.0).contains(&value) {
                return Err(format!("{name}必须在 0 到 10 秒之间"));
            }
        }
        if !self.overlay_opacity.is_finite() || !(0.3..=1.0).contains(&self.overlay_opacity) {
            return Err("悬浮窗透明度必须在 30% 到 100% 之间".into());
        }
        self.hotkeys.bindings()?;
        self.game_keys.applied()?;
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

    pub fn first_hip_offset(&self) -> [i32; 2] {
        scale_and_trim(self.first_hip_base, self.look_scale(), [0, 0])
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
    fn default_offsets_match_calibrated_reference() {
        let config = AppConfig::default();
        assert_eq!(config.first_aim_mode, FirstAimMode::Ads);
        assert_eq!(config.first_ads_offset(), [-2600, 50]);
        assert_eq!(config.first_hip_offset(), [-1320, 30]);
        assert_eq!(config.void_arrow_offset(), [-350, 81]);
        assert_eq!(config.sprint_offset(), [280, 0]);
        assert_eq!(config.timings.strafe_to_flag_wait, 0.6);
        assert_eq!(config.timings.flag_to_claim_wait, 1.0);
        assert_eq!(config.timings.claim_to_weapon_wait, 0.2);
        assert_eq!(config.timings.weapon_to_move_wait, 0.5);
        assert_eq!(config.timings.position_to_aim_wait, 1.5);
        assert_eq!(config.timings.aim_to_melee_wait, 0.9);
        assert_eq!(config.timings.melee_to_super_wait, 0.15);
        assert_eq!(config.timings.super_to_sprint_wait, 2.0);
        assert_eq!(config.timings.sprint_to_finisher, 0.0);
    }

    #[test]
    fn legacy_timing_settings_migrate_to_adjacent_action_waits() {
        let config: AppConfig = serde_json::from_str(
            r#"{
                "timings": {
                    "ascensionWait": 2.1,
                    "meleeExtraWait": 0.4,
                    "adsToSuperWait": 2.8,
                    "superWait": 2.2,
                    "sprintATime": 0.2,
                    "sprintToFinisher": 0.15
                }
            }"#,
        )
        .unwrap();

        assert_eq!(config.timings.strafe_to_flag_wait, 0.6);
        assert_eq!(config.timings.aim_to_melee_wait, 0.922);
        assert!((config.timings.melee_to_super_wait - 0.384).abs() < 1e-9);
        assert_eq!(config.timings.super_to_sprint_wait, 2.2);
        assert_eq!(config.timings.sprint_to_finisher, 0.15);
        assert!(config.validate().is_ok());
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
        assert_eq!(config.first_hip_offset(), [-1980, 45]);
        assert_eq!(config.void_arrow_offset(), [-445, 119]);
    }

    #[test]
    fn fov_does_not_change_world_turn_mouse_counts() {
        let reference = AppConfig {
            first_hip_base: [-1300, 25],
            field_of_view: 100.0,
            ..AppConfig::default()
        };
        let wider_fov = AppConfig {
            field_of_view: 105.0,
            ..reference.clone()
        };
        assert_eq!(reference.first_hip_offset(), [-1300, 25]);
        assert_eq!(wider_fov.first_hip_offset(), [-1300, 25]);
        assert_eq!(wider_fov.first_ads_offset(), reference.first_ads_offset());
        assert_eq!(wider_fov.void_arrow_offset(), reference.void_arrow_offset());
        assert_eq!(wider_fov.sprint_offset(), reference.sprint_offset());
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
        assert_eq!(config.first_aim_mode, FirstAimMode::Ads);
        assert_eq!(config.first_hip_base, [-1320, 30]);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn hipfire_mode_round_trips_through_settings_json() {
        let config = AppConfig {
            first_aim_mode: FirstAimMode::Hipfire,
            first_hip_base: [-1580, 29],
            ..AppConfig::default()
        };
        let raw = serde_json::to_string(&config).unwrap();
        let decoded: AppConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(decoded.first_aim_mode, FirstAimMode::Hipfire);
        assert_eq!(decoded.first_hip_base, [-1580, 29]);
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
        assert!(config.hotkeys.bindings().is_ok());
        assert_eq!(
            config.game_keys.applied().unwrap().all(),
            [
                input::binding("ShiftLeft").unwrap(),
                input::binding("Space").unwrap(),
                input::binding("KeyE").unwrap(),
                input::binding("Digit2").unwrap(),
                input::binding("KeyC").unwrap(),
                input::binding("KeyX").unwrap(),
                input::binding("KeyF").unwrap(),
                input::binding("KeyG").unwrap(),
            ]
        );
    }

    #[test]
    fn mouse_side_buttons_are_valid_hotkeys_and_game_inputs() {
        let config = AppConfig {
            hotkeys: HotkeyConfig {
                start: "Control+Mouse4".into(),
                stop: "Mouse5".into(),
            },
            game_keys: GameKeyConfig {
                melee: "Mouse4".into(),
                finisher: "Mouse5".into(),
                ..GameKeyConfig::default()
            },
            ..AppConfig::default()
        };
        assert!(config.validate().is_ok());
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
    fn role_binding_parses_each_hotkey_independently() {
        let config = HotkeyConfig {
            start: "F8".into(),
            stop: "Control+Mouse4".into(),
        };
        assert_eq!(
            config.role_binding(HotkeyRole::Start),
            Ok(HotkeyBinding::Keyboard("F8".parse().unwrap()))
        );
        assert_eq!(
            config.role_binding(HotkeyRole::Stop),
            Ok(HotkeyBinding::Mouse(MouseShortcut {
                button: MouseButton::Back,
                modifiers: input::MOD_CONTROL,
            }))
        );
    }

    #[test]
    fn role_binding_reports_raw_parse_errors() {
        let config = HotkeyConfig {
            start: "NotAKey".into(),
            stop: "F10".into(),
        };
        assert!(config
            .role_binding(HotkeyRole::Start)
            .unwrap_err()
            .contains("NotAKey"));
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
