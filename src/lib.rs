use anyhow::Context;
use aviutl2_eframe::egui::TextBuffer;

mod curve;
mod gui;
mod module;

#[aviutl2::plugin(GenericPlugin)]
struct KeyframesAux2 {
    mod2: aviutl2::generic::SubPlugin<crate::module::KeyframesMod2>,
    gui: aviutl2_eframe::EframeWindow,
}

pub static EFFECTS: std::sync::LazyLock<dashmap::DashMap<String, aviutl2::generic::Effect>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();
pub static OBJECT_ID_TO_HANDLE: std::sync::LazyLock<
    dashmap::DashMap<usize, aviutl2::generic::ObjectHandle>,
> = std::sync::LazyLock::new(dashmap::DashMap::new);
pub static KEYFRAMES: std::sync::LazyLock<
    dashmap::DashMap<KeyframeTrackParams, crate::curve::Keyframes>,
> = std::sync::LazyLock::new(dashmap::DashMap::new);
pub static CURRENT_BANK: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(1));
pub static CURRENT_KEYFRAMES_ID: std::sync::LazyLock<std::sync::Mutex<usize>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(0));
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyframeTrackParams {
    pub bank_id: usize,
    pub keyframes_id: usize,
}

impl KeyframeTrackParams {
    pub fn new() -> Self {
        let current_bank_id = CURRENT_BANK.lock().unwrap();
        let mut current_keyframes_id = CURRENT_KEYFRAMES_ID.lock().unwrap();
        let params = Self {
            bank_id: *current_bank_id,
            keyframes_id: *current_keyframes_id,
        };
        *current_keyframes_id += 1;
        params
    }
}

impl Default for KeyframeTrackParams {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyframeBinding {
    pub object: aviutl2::generic::ObjectHandle,
    pub effect_name: String,
    pub effect_index: usize,
    pub track_name: String,
}

impl KeyframeTrackParams {
    pub fn parse(alias: &str) -> Option<Self> {
        static KEYFRAME_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> = lazy_regex::lazy_regex!(
            r"keyframes\.aux2,\d+\|(?<bank_id>\d+),(?<keyframes_id>\d+)(?:$|\|)"
        );
        let captures = KEYFRAME_PATTERN.captures(alias)?;
        let bank_id: usize = captures.name("bank_id")?.as_str().parse().ok()?;
        let keyframes_id: usize = captures.name("keyframes_id")?.as_str().parse().ok()?;
        Some(Self {
            bank_id,
            keyframes_id,
        })
    }
    pub fn set_params(&self, track: &mut String) -> anyhow::Result<()> {
        static STATIC_VALUE_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> =
            lazy_regex::lazy_regex!(r"^[0-9\.]+$");
        if STATIC_VALUE_PATTERN.is_match(track) {
            track.replace_with(
                &format!(
                    "{},{},keyframes.aux2,0|{},{}|",
                    track, track, self.bank_id, self.keyframes_id
                ),
            );
            return Ok(());
        }
        static KEYFRAME_PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> =
            lazy_regex::lazy_regex!(r"(?<easing>[^,]+),(?<flags>\d+)(?<rest>$|\|[^|]*$|\|[^|]*\|)");
        let captures = KEYFRAME_PATTERN
            .captures(track)
            .context("Failed to match keyframe alias pattern")?;
        let flags = captures.name("flags").unwrap();
        let rest = captures.name("rest").unwrap();
        let new_alias = format!(
            "keyframes.aux2,{}|{},{}{}",
            flags.as_str(),
            self.bank_id,
            self.keyframes_id,
            rest.as_str()
        );
        let start = captures.get(0).unwrap().start();
        let end = captures.get(0).unwrap().end();
        track.replace_range(start..end, &new_alias);
        Ok(())
    }
}

impl aviutl2::generic::GenericPlugin for KeyframesAux2 {
    fn new(info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
        aviutl2::lprintln!(
            "Config initialized?: {:?}",
            aviutl2::config::app_data_path()
        );
        Ok(Self {
            mod2: aviutl2::generic::SubPlugin::new_script_module(&info)?,
            gui: aviutl2_eframe::EframeWindow::new("keyframes.aux2", crate::gui::create_gui)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "keyframes.aux2".into(),
            information: "".into(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.register_script_module(Some("keyframes.aux2"), &self.mod2);
        let handle = registry.create_edit_handle();
        let window = handle.get_host_app_window_raw().unwrap();
        match self.gui.handle() {
            Ok(handle) => {
                self.gui.egui_ctx().unwrap().set_pixels_per_point(unsafe {
                    windows::Win32::UI::HiDpi::GetDpiForWindow(windows::Win32::Foundation::HWND(
                        window.hwnd.get() as *mut std::ffi::c_void,
                    )) as f32
                        / 96.0
                });
                let _ = registry.register_window_client("keyframes.aux2", &handle);
            }
            Err(e) => {
                tracing::error!("Failed to register GUI window: {:?}", e);
            }
        }
        EDIT_HANDLE.init(handle);
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        if EFFECTS.is_empty() {
            tracing::info!("Loading effects...");
            let effects = EDIT_HANDLE.get_effects();
            for effect in effects {
                EFFECTS.insert(effect.name.clone(), effect);
            }
            tracing::info!("Loaded {} effects", EFFECTS.len());
        }

        let last_bank_id: usize = project.deserialize("last_bank_id").unwrap_or(0);
        {
            let mut current_bank = CURRENT_BANK.lock().unwrap();
            *current_bank = last_bank_id + 1;
        }
        let keyframes: Vec<(KeyframeTrackParams, crate::curve::Keyframes)> =
            project.deserialize("keyframes").unwrap_or_default();
        KEYFRAMES.clear();
        for (params, keyframes) in keyframes.into_iter() {
            KEYFRAMES.insert(params, keyframes);
        }
    }

    fn on_change_scene(&mut self, edit: &aviutl2::generic::EditSection) {
        {
            let mut current_bank_id = CURRENT_BANK.lock().unwrap();
            *current_bank_id += 1;
        }
        let mut used_bank_ids = std::collections::HashSet::new();
        let mut used_keyframes = std::collections::HashSet::new();
        for layer in edit.layers() {
            for (position, object) in layer.objects() {
                if let Err(e) =
                    collect_used_keyframes(edit, object, &mut used_bank_ids, &mut used_keyframes)
                {
                    tracing::error!(
                        "Failed to collect used keyframes for object at position {:?} in layer {:?}: {:?}",
                        position,
                        layer.index,
                        e
                    );
                }
            }
        }
        tracing::info!("Used bank IDs: {:?}", used_bank_ids);
        tracing::info!("Used keyframes: {:?}", used_keyframes);
        let before_len = KEYFRAMES.len();
        KEYFRAMES.retain(|params, _| {
            !used_bank_ids.contains(&params.bank_id) || used_keyframes.contains(params)
        });
        tracing::info!("Removed {} unused keyframes", before_len - KEYFRAMES.len());
    }
}

fn collect_used_keyframes(
    edit: &aviutl2::generic::EditSection,
    object: aviutl2::generic::ObjectHandle,
    used_bank_ids: &mut std::collections::HashSet<usize>,
    used_keyframes: &mut std::collections::HashSet<KeyframeTrackParams>,
) -> anyhow::Result<()> {
    let alias = edit
        .get_object_alias_parsed(object)
        .context("Failed to get object alias")?;
    let objects = alias
        .get_table("Object")
        .context("Failed to get Object table")?;
    for object in objects.iter_subtables_as_array() {
        let effect_name = object
            .get_value("effect.name")
            .context("Failed to get effect name")?;
        crate::EDIT_HANDLE.enumerate_effect_items(effect_name, |item| {
            if item.item_type != aviutl2::generic::EffectItemType::Number {
                return;
            }
            let Some(value) = object.get_value(&item.name) else {
                return;
            };
            let Some(params) = crate::KeyframeTrackParams::parse(value) else {
                return;
            };
            used_bank_ids.insert(params.bank_id);
            used_keyframes.insert(params);
        })?;
    }
    Ok(())
}

aviutl2::register_generic_plugin!(KeyframesAux2);
