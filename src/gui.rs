use anyhow::Context;
use aviutl2_eframe::{eframe, egui};
use tap::prelude::*;

pub struct KeyframesGui {
    selected_object_info: Option<SelectedObjectInfo>,
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
    name: String,
    frames: Vec<usize>,
    effects: Vec<EffectInfo>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    name: String,
    effect_type: EffectType,
    keyframe_tracks: indexmap::IndexMap<crate::KeyframeTrackParams, KeyframeTrackInfo>,
}

#[derive(Debug, Clone)]
pub struct KeyframeTrackInfo {
    names: Vec<String>,
}

fn get_colors(object_type: &EffectType) -> (Vec<egui::Color32>, egui::Color32) {
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
    }))
}

impl aviutl2_eframe::eframe::App for KeyframesGui {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if crate::EDIT_HANDLE.is_ready() {
            let _ = crate::EDIT_HANDLE.call_read_section(|r| {
                let res = Self::update_keyframe_bindings(r);
                if let Err(e) = res {
                    tracing::error!("Failed to update keyframe bindings: {:?}", e);
                }
                let res = self.update_selected_object_info(r);
                if let Err(e) = res {
                    tracing::error!("Failed to update selected object info: {:?}", e);
                }
            });
        }
    }
    fn ui(&mut self, ui: &mut aviutl2_eframe::egui::Ui, frame: &mut aviutl2_eframe::eframe::Frame) {
        ui.request_repaint_after(std::time::Duration::from_millis(100));
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if crate::EDIT_HANDLE.is_ready() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_selected_object_info(ui);
                });
            } else {
                ui.label("Initializing...");
            }
        });
    }
}

impl KeyframesGui {
    fn update_keyframe_bindings(
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<()> {
        let info = crate::EDIT_HANDLE.get_edit_info();
        let mut bindings = std::collections::HashMap::<
            crate::KeyframeTrackParams,
            Vec<crate::KeyframeBinding>,
        >::new();

        for layer in 0..=info.layer_max {
            for (_, object) in read.objects_in_layer(layer) {
                Self::collect_object_keyframe_bindings(read, object, &mut bindings)?;
            }
        }

        crate::PARAMS_TO_BINDINGS.clear();
        for (params, bindings) in bindings {
            crate::PARAMS_TO_BINDINGS.insert(params, bindings);
        }

        Ok(())
    }

    fn collect_object_keyframe_bindings(
        read: &aviutl2::generic::ReadSection,
        object_handle: aviutl2::generic::ObjectHandle,
        bindings: &mut std::collections::HashMap<
            crate::KeyframeTrackParams,
            Vec<crate::KeyframeBinding>,
        >,
    ) -> aviutl2::common::AnyResult<()> {
        let alias = read
            .get_object_alias_parsed(object_handle)
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
                bindings
                    .entry(params)
                    .or_default()
                    .push(crate::KeyframeBinding {
                        object: object_handle,
                        effect_name: effect_name.to_string(),
                        track_name: item.name,
                    });
            })?;
        }

        Ok(())
    }

    fn render_selected_object_info(&self, ui: &mut egui::Ui) {
        let Some(selected_object_info) = &self.selected_object_info else {
            ui.label("No object selected");
            return;
        };
        ui.label(format!("Selected Object: {}", selected_object_info.name));
        let info = crate::EDIT_HANDLE.get_edit_info();
        for effect in &selected_object_info.effects {
            self.render_effect_info(ui, &info, selected_object_info, effect);
        }
    }

    fn render_effect_info(
        &self,
        ui: &mut egui::Ui,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
    ) {
        ui.collapsing(format!("Effect: {}", effect.name), |ui| {
            for (_, track) in &effect.keyframe_tracks {
                self.render_keyframe_track_info(ui, info, object, effect, track);
            }
        });
    }

    fn render_keyframe_track_info(
        &self,
        ui: &mut egui::Ui,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
    ) {
        ui.label(track.names.join(", "));
        let (response, painter) = ui.allocate_painter(
            ui.available_size().tap_mut(|s| {
                s.y = 24.0;
            }),
            aviutl2_eframe::egui::Sense::hover(),
        );
        let (current_object_color, selected_object_color) = get_colors(&effect.effect_type);
        let num_divisions = response.rect.width() as usize / 10;
        if num_divisions == 0 {
            return;
        }
        let width_per_section = response.rect.width() / num_divisions as f32;
        for i in 0..num_divisions {
            let mut rect = response.rect;
            rect.set_left(rect.left() + i as f32 * width_per_section);
            rect.set_right(rect.left() + width_per_section);
            let position = i as f32 / num_divisions as f32;
            let color = current_object_color[position.floor() as usize].lerp_to_gamma(
                current_object_color
                    [(position.ceil() as usize).min(current_object_color.len() - 1)],
                position.fract(),
            );
            painter.rect_filled(rect, 0.0, color);
        }
        let total_frames = object.frames.last().unwrap() - object.frames.first().unwrap() + 1;
        for (i, frame) in object.frames.iter().enumerate() {
            if i == 0 || i == object.frames.len() - 1 {
                continue;
            }
            let position = (*frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            let mut rect = response.rect;
            rect.set_left(rect.left() + position * response.rect.width() - 1.0);
            rect.set_right(rect.left() + 1.0);
            painter.rect_filled(rect, 0.0, ui.visuals().widgets.noninteractive.bg_fill);
        }

        if *object.frames.first().unwrap() <= info.frame
            && info.frame <= *object.frames.last().unwrap()
        {
            let position =
                (info.frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            let mut rect = response.rect;
            rect.set_left(rect.left() + position * response.rect.width() - 1.0);
            rect.set_right(rect.left() + 1.0);
            let selected_line = aviutl2::config::get_color_code("FrameCursor")
                .expect("Null文字はない")
                .expect("そもそもこれが落ちるなら本体も落ちる")
                .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b));
            painter.rect_filled(rect, 0.0, selected_line);
        }
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
        for object in objects.iter_subtables_as_array() {
            let effect_name = object
                .get_value("effect.name")
                .context("Failed to get effect name")?;

            let effect_info = crate::EFFECTS
                .get(effect_name)
                .context("Failed to get effect info")?;
            let effect_type = Self::determine_effect_type(&effect_info, Some(first_effect_type));

            let mut effect_info = EffectInfo {
                name: effect_name.to_string(),
                effect_type,
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
                if effect_info.flag.video {
                    EffectType::VideoFilter
                } else if effect_info.flag.audio {
                    EffectType::AudioFilter
                } else {
                    tracing::error!(
                        "Effect with Filter type but no video/audio flag: {:?}",
                        effect_info
                    );
                    unreachable!()
                }
            }
            aviutl2::generic::EffectType::Filter if effect_info.flag.video => {
                EffectType::VideoEffect
            }
            aviutl2::generic::EffectType::Filter if effect_info.flag.audio => {
                EffectType::AudioEffect
            }
            aviutl2::generic::EffectType::Filter => {
                tracing::error!(
                    "Effect with Filter type but no video/audio flag: {:?}",
                    effect_info
                );
                unreachable!()
            }

            aviutl2::generic::EffectType::Input if effect_info.flag.video => EffectType::VideoInput,
            aviutl2::generic::EffectType::Input if effect_info.flag.audio => EffectType::AudioInput,
            aviutl2::generic::EffectType::Input => {
                tracing::error!(
                    "Effect with Input type but no video/audio flag: {:?}",
                    effect_info
                );
                unreachable!()
            }
            aviutl2::generic::EffectType::SceneChange => EffectType::Control,
            aviutl2::generic::EffectType::Control => EffectType::Control,
            aviutl2::generic::EffectType::Output => {
                first_effect_type.unwrap_or(EffectType::Control)
            }
        }
    }
}
