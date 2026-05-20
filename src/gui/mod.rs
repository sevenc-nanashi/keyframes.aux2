use anyhow::Context;
use aviutl2_eframe::{eframe, egui};
use tap::prelude::*;

mod keyframes;
mod timecontrol;

pub struct KeyframesGui {
    pub(super) selected_object_info: Option<SelectedObjectInfo>,
    pub(super) timecontrol_editor: Option<TimeControlEditorTarget>,
    pub(super) debug_counter: usize,
}

#[derive(Debug, Clone)]
pub(super) struct TimeControlEditorTarget {
    pub(super) params: crate::KeyframeTrackParams,
    pub(super) keyframe_index: usize,
    pub(super) object: aviutl2::generic::ObjectHandle,
    pub(super) effect_name: String,
    pub(super) effect_index: usize,
    pub(super) track_names: Vec<String>,
    pub(super) timecontrol: crate::curve::TimeControlBezier,
    pub(super) selected_point: usize,
    pub(super) context_menu_position: Option<[f64; 2]>,
    pub(super) dirty: bool,
}

#[derive(Debug, Clone, Copy)]
enum TimeControlHandleKind {
    In,
    Out,
}
impl TimeControlHandleKind {
    fn id(self) -> &'static str {
        match self {
            TimeControlHandleKind::In => "in",
            TimeControlHandleKind::Out => "out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    Control,
    VideoInput,
    VideoEffect,
    VideoFilter,
    AudioInput,
    AudioEffect,
    AudioFilter,
}

#[derive(Debug, Clone)]
pub struct SelectedObjectInfo {
    pub(super) handle: aviutl2::generic::ObjectHandle,
    pub(super) name: String,
    pub(super) frames: Vec<usize>,
    pub(super) effects: Vec<EffectInfo>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    pub(super) name: String,
    pub(super) index: usize,
    pub(super) effect_type: EffectType,
    pub(super) keyframe_tracks: indexmap::IndexMap<crate::KeyframeTrackParams, KeyframeTrackInfo>,
}

#[derive(Debug, Clone)]
pub struct KeyframeTrackInfo {
    pub(super) names: Vec<String>,
}

pub(super) fn get_colors(object_type: &EffectType) -> (Vec<egui::Color32>, egui::Color32) {
    let (normal, selected) = match object_type {
        EffectType::Control => ("ObjectControl", "ObjectControlSelect"),
        EffectType::VideoInput => ("ObjectVideo", "ObjectVideoSelect"),
        EffectType::VideoEffect => ("ObjectVideoEffect", "ObjectVideoEffectSelect"),
        EffectType::VideoFilter => ("ObjectVideoFilter", "ObjectVideoFilterSelect"),
        EffectType::AudioInput => ("ObjectAudio", "ObjectAudioSelect"),
        EffectType::AudioEffect => ("ObjectAudioEffect", "ObjectAudioEffectSelect"),
        EffectType::AudioFilter => ("ObjectAudioFilter", "ObjectAudioFilterSelect"),
    };
    let normal_color =
        aviutl2::config::get_all_color_codes(normal).expect("そもそもこれが落ちるなら本体も落ちる");
    let selected_color = aviutl2::config::get_color_code(selected)
        .expect("Null文字はない")
        .expect("そもそもこれが落ちるなら本体も落ちる");
    let selected_color =
        egui::Color32::from_rgb(selected_color.0, selected_color.1, selected_color.2);
    if normal_color.len() == 1 {
        let normal_color =
            egui::Color32::from_rgb(normal_color[0].0, normal_color[0].1, normal_color[0].2);
        (vec![normal_color, normal_color], selected_color)
    } else {
        (
            normal_color
                .into_iter()
                .map(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
                .collect(),
            selected_color,
        )
    }
}

pub fn create_gui(
    cc: &aviutl2_eframe::eframe::CreationContext,
    _handle: aviutl2_eframe::AviUtl2EframeHandle,
) -> Result<Box<dyn aviutl2_eframe::eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    cc.egui_ctx.all_styles_mut(|style| {
        style.visuals = aviutl2_eframe::aviutl2_visuals();
    });
    cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());
    Ok(Box::new(KeyframesGui {
        selected_object_info: None,
        timecontrol_editor: None,
        debug_counter: 0,
    }))
}

static RESOLVED_MIGRATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<crate::KeyframeTrackParams>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

impl aviutl2_eframe::eframe::App for KeyframesGui {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if crate::EDIT_HANDLE.is_ready() {
            if !crate::EDIT_HANDLE
                .get_edit_state()
                .is_ok_and(|state| state == aviutl2::generic::EditState::Edit)
            {
                return;
            }
            let update_bindings = crate::EDIT_HANDLE
                .call_read_section(Self::update_keyframe_bindings)
                .map_err(anyhow::Error::from)
                .flatten();
            let change_bindings = match update_bindings {
                Ok(bindings) => bindings,
                Err(e) => {
                    tracing::error!("Failed to update keyframe bindings: {:?}", e);
                    return;
                }
            };
            if !change_bindings.is_empty() {
                tracing::info!(
                    "Updating keyframe track params for {} bindings",
                    change_bindings.len()
                );
                let update_result = crate::EDIT_HANDLE
                    .call_edit_section(|edit| {
                        let mut resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
                        for (binding, new_params) in change_bindings {
                            tracing::info!(
                                "Updating keyframe track params for object {:?}, effect {:?} (index {}), track {:?} to {:?}",
                                binding.object,
                                binding.effect_name,
                                binding.effect_index,
                                binding.track_name,
                                new_params
                            );
                            let mut track = edit.get_object_effect_item(
                                binding.object,
                                &binding.effect_name,
                                binding.effect_index,
                                &binding.track_name,
                            )?;
                            tracing::debug!(
                                "Current keyframe track params for object {:?}, effect {:?} (index {}), track {:?}: {:?}",
                                binding.object,
                                binding.effect_name,
                                binding.effect_index,
                                binding.track_name,
                                &track
                            );
                            let previous_params = crate::KeyframeTrackParams::parse(&track);
                            if let Some(previous_params) = previous_params {
                                resolved_migrations.insert(previous_params);
                            }
                            new_params.set_params(&mut track)?;
                            edit.set_object_effect_item(
                                binding.object,
                                &binding.effect_name,
                                binding.effect_index,
                                &binding.track_name,
                                &track,
                            )?;
                            tracing::debug!(
                                "Updated keyframe track params for object {:?}, effect {:?} (index {}), track {:?} to {:?}",
                                binding.object,
                                binding.effect_name,
                                binding.effect_index,
                                binding.track_name,
                                &track
                            );
                        }
                        anyhow::Ok(())
                    })
                    .map_err(anyhow::Error::from)
                    .flatten();
                if let Err(e) = update_result {
                    tracing::error!("Failed to update keyframe track params: {:?}", e);
                    return;
                }
            }

            let update_selected_object_info = crate::EDIT_HANDLE
                .call_read_section(|read| {
                    self.update_selected_object_info(read)?;
                    self.update_timecontrol_editor_target(read)?;
                    anyhow::Ok(())
                })
                .map_err(anyhow::Error::from)
                .flatten();
            if let Err(e) = update_selected_object_info {
                tracing::error!("Failed to update selected object info: {:?}", e);
            }
        }
    }
    fn ui(
        &mut self,
        ui: &mut aviutl2_eframe::egui::Ui,
        _frame: &mut aviutl2_eframe::eframe::Frame,
    ) {
        ui.request_repaint_after(std::time::Duration::from_millis(100));
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if crate::EDIT_HANDLE.is_ready() {
                if self.is_undo_mode() {
                    self.render_undo_mode_warning(ui);
                }
                if self.timecontrol_editor.is_some() {
                    self.render_timecontrol_editor(ui);
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_selected_object_info(ui);
                    });
                }
            } else {
                ui.label("Initializing...");
            }
        });
    }
}

impl KeyframesGui {
    fn is_undo_mode(&self) -> bool {
        let Some(selected_object_info) = &self.selected_object_info else {
            return false;
        };

        selected_object_info.effects.iter().any(|effect| {
            effect.keyframe_tracks.keys().any(|params| {
                crate::KEYFRAMES.get(params).is_some_and(|keyframes| {
                    keyframes.keyframes.len() != selected_object_info.frames.len()
                })
            })
        })
    }

    fn render_undo_mode_warning(&self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), aviutl2_eframe::egui::Sense::click());
        let rect = response.rect;

        if response.clicked() {
            let mut resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
            resolved_migrations.clear();
        }

        let color = aviutl2::config::get_color_code("LogWarn")
            .expect("Null文字はない")
            .expect("そもそもこれが落ちるなら本体も落ちる")
            .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b));

        let mut layout = egui::text::LayoutJob::default();
        layout.append(
            "一時停止中",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(18.0),
                color,
                ..Default::default()
            },
        );
        layout.append(
            "Undoを妨げないために同期を中断しています。クリックで再同期します。",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::default(),
                color: ui.visuals().widgets.noninteractive.fg_stroke.color,
                ..Default::default()
            },
        );
        layout.wrap = egui::text::TextWrapping::wrap_at_width(rect.width());
        let galley = painter.layout_job(layout);
        painter.galley(
            rect.center().tap_mut(|pos| {
                pos.x -= galley.size().x / 2.0;
                pos.y -= galley.size().y / 2.0;
            }),
            galley,
            color,
        );
    }

    fn update_keyframe_bindings(
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<
        indexmap::IndexMap<crate::KeyframeBinding, crate::KeyframeTrackParams>,
    > {
        let info = crate::EDIT_HANDLE.get_edit_info();
        let mut bindings =
            indexmap::IndexMap::<crate::KeyframeTrackParams, Vec<crate::KeyframeBinding>>::new();

        for layer in 0..=info.layer_max {
            for (_, object) in read.objects_in_layer(layer) {
                Self::collect_object_keyframe_bindings(read, object, &mut bindings)?;
            }
        }

        let mut change_bindings =
            indexmap::IndexMap::<crate::KeyframeBinding, crate::KeyframeTrackParams>::new();
        let mut migrations = std::collections::HashMap::<
            crate::KeyframeTrackParams,
            crate::KeyframeTrackParams,
        >::new();
        let mut param_to_effect = indexmap::IndexMap::<
            crate::KeyframeTrackParams,
            (aviutl2::generic::ObjectHandle, String, usize),
        >::new();
        let resolved_migrations = RESOLVED_MIGRATIONS.lock().unwrap();
        for (params, bindings) in &bindings {
            for binding in bindings {
                let effect_key = (
                    binding.object,
                    binding.effect_name.clone(),
                    binding.effect_index,
                );
                if resolved_migrations.contains(params) {
                    continue;
                }
                if let Some(existing_params) = param_to_effect.get(params)
                    && existing_params != &effect_key
                {
                    tracing::info!(
                        "Duplicated keyframe track params {:?} for effect {:?} and effect {:?}",
                        params,
                        existing_params,
                        effect_key
                    );
                    let new_params = *migrations.entry(*params).or_default();
                    change_bindings.insert(binding.clone(), new_params);
                    if let Some(keyframes) = crate::KEYFRAMES.get(params) {
                        crate::KEYFRAMES.insert(new_params, keyframes.clone());
                    }
                    migrations.insert(*params, new_params);
                } else if params.bank_id == 0 {
                    tracing::info!(
                        "Uninitialized keyframe track params {:?} for effect {:?}",
                        params,
                        effect_key
                    );
                    let new_params = crate::KeyframeTrackParams::new();
                    change_bindings.insert(binding.clone(), new_params);
                } else {
                    let num_keyframes = read.get_object_section_num(binding.object)? + 1;
                    match crate::KEYFRAMES.get(params) {
                        None => {
                            tracing::info!(
                                "Keyframe track params {:?} for effect {:?} is not registered in global keyframes map",
                                params,
                                effect_key
                            );
                            crate::KEYFRAMES
                                .insert(*params, crate::curve::Keyframes::new(num_keyframes));
                            param_to_effect.insert(*params, effect_key);
                        }
                        Some(existing_keyframes)
                            if existing_keyframes.keyframes.len() != num_keyframes =>
                        {
                            tracing::info!(
                                "Keyframe track params {:?} for effect {:?} has different number of keyframes ({} in global map, {} in object)",
                                params,
                                effect_key,
                                existing_keyframes.keyframes.len(),
                                num_keyframes
                            );
                            let new_params = *migrations.entry(*params).or_default();
                            let mut new_keyframes = existing_keyframes.clone();
                            drop(existing_keyframes);
                            new_keyframes.resize(num_keyframes);
                            crate::KEYFRAMES.insert(new_params, new_keyframes);
                            change_bindings.insert(binding.clone(), new_params);
                            param_to_effect.insert(*params, effect_key);
                            migrations.insert(*params, new_params);
                        }
                        Some(_) => {
                            param_to_effect.insert(*params, effect_key);
                        }
                    };
                }
            }
        }

        Ok(change_bindings)
    }

    fn collect_object_keyframe_bindings(
        read: &aviutl2::generic::ReadSection,
        object_handle: aviutl2::generic::ObjectHandle,
        bindings: &mut indexmap::IndexMap<crate::KeyframeTrackParams, Vec<crate::KeyframeBinding>>,
    ) -> aviutl2::common::AnyResult<()> {
        let alias = read
            .get_object_alias_parsed(object_handle)
            .context("Failed to get object alias")?;
        let objects = alias
            .get_table("Object")
            .context("Failed to get Object table")?;

        let mut effect_count = std::collections::HashMap::<String, usize>::new();
        for object in objects.iter_subtables_as_array() {
            let effect_name = object
                .get_value("effect.name")
                .context("Failed to get effect name")?;
            let effect_index = effect_count.entry(effect_name.to_string()).or_insert(0);
            *effect_index += 1;
            let effect_index = *effect_index - 1;
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
                bindings
                    .entry(params)
                    .or_default()
                    .push(crate::KeyframeBinding {
                        object: object_handle,
                        effect_name: effect_name.to_string(),
                        effect_index,
                        track_name: item.name,
                    });
            })?;
        }

        Ok(())
    }

    fn update_selected_object_info(
        &mut self,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<()> {
        let selected_object = read.get_focused_object()?;
        let Some(selected_object) = selected_object else {
            self.selected_object_info = None;
            return Ok(());
        };
        let alias = read
            .get_object_alias_parsed(selected_object)
            .context("Failed to get object alias")?;
        let objects = alias
            .get_table("Object")
            .context("Failed to get Object table")?;
        let first_effect = objects
            .get_table("0")
            .context("Failed to get first object in Object table")?;
        let first_effect_name = first_effect
            .get_value("effect.name")
            .context("Failed to get effect name")?;
        let first_effect_info = crate::EFFECTS
            .get(first_effect_name)
            .context("Failed to get effect info")?;
        let first_effect_type = Self::determine_effect_type(&first_effect_info, None);
        let mut effects = Vec::new();
        let mut effect_count = std::collections::HashMap::<String, usize>::new();
        for object in objects.iter_subtables_as_array() {
            let effect_name = object
                .get_value("effect.name")
                .context("Failed to get effect name")?;

            let effect_info = crate::EFFECTS
                .get(effect_name)
                .context("Failed to get effect info")?;
            let effect_type = Self::determine_effect_type(&effect_info, Some(first_effect_type));
            let effect_index = effect_count.entry(effect_name.to_string()).or_insert(0);
            *effect_index += 1;
            let effect_index = *effect_index - 1;

            let mut effect_info = EffectInfo {
                name: effect_name.to_string(),
                effect_type,
                index: effect_index,
                keyframe_tracks: indexmap::IndexMap::new(),
            };
            crate::EDIT_HANDLE.enumerate_effect_items(effect_name, |item| {
                if item.item_type != aviutl2::generic::EffectItemType::Number {
                    return;
                }
                // NOTE:
                // エフェクトごとのカウンターとかが面倒なのでEffectItemはitem_typeのチェックでしか使わない
                let Some(value) = object.get_value(&item.name) else {
                    tracing::error!(
                        "Failed to get value for effect item {:?} in effect {:?}",
                        item.name,
                        effect_name
                    );
                    return;
                };
                if let Some(params) = crate::KeyframeTrackParams::parse(value) {
                    // let keyframe_info = KeyframeTrackInfo {
                    //     name: key.to_string(),
                    //     params,
                    // };
                    // effect_info.keyframe_tracks.push(keyframe_info);
                    effect_info
                        .keyframe_tracks
                        .entry(params)
                        .or_insert_with(|| KeyframeTrackInfo { names: Vec::new() })
                        .names
                        .push(item.name.to_string());
                }
            })?;
            effects.push(effect_info);
        }

        let frames = objects
            .get_value("frame")
            .context("Failed to get frame value")?
            .split(',')
            .filter_map(|s| s.parse::<usize>().ok())
            .collect::<Vec<_>>();
        let selected_object_info = SelectedObjectInfo {
            handle: selected_object,
            name: read.get_object_name(selected_object)?.unwrap_or_else(|| {
                effects
                    .iter()
                    .find(|e| e.effect_type != EffectType::Control)
                    .map(|e| e.name.clone())
                    .or_else(|| effects.first().map(|e| e.name.clone()))
                    .unwrap_or_else(|| "Unknown Object".to_string())
            }),
            frames,
            effects,
        };
        self.selected_object_info = Some(selected_object_info);

        Ok(())
    }

    fn determine_effect_type(
        effect_info: &aviutl2::generic::Effect,
        first_effect_type: Option<EffectType>,
    ) -> EffectType {
        match effect_info.effect_type {
            aviutl2::generic::EffectType::Filter
                if matches!(first_effect_type, Some(EffectType::Control)) =>
            {
                if effect_info.flag.audio {
                    EffectType::AudioFilter
                } else {
                    EffectType::VideoFilter
                }
            }
            aviutl2::generic::EffectType::Filter if effect_info.flag.audio => {
                EffectType::AudioEffect
            }
            aviutl2::generic::EffectType::Filter => EffectType::VideoEffect,

            aviutl2::generic::EffectType::Input if effect_info.flag.audio => EffectType::AudioInput,
            aviutl2::generic::EffectType::Input => EffectType::VideoInput,
            aviutl2::generic::EffectType::SceneChange => EffectType::Control,
            aviutl2::generic::EffectType::Control => EffectType::Control,
            aviutl2::generic::EffectType::Output => {
                first_effect_type.unwrap_or(EffectType::Control)
            }
        }
    }
}
