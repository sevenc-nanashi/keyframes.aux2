use anyhow::Context;
use aviutl2_eframe::{eframe, egui};
use tap::prelude::*;

pub struct KeyframesGui {
    selected_object_info: Option<SelectedObjectInfo>,
    debug_counter: usize,
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
    handle: aviutl2::generic::ObjectHandle,
    name: String,
    frames: Vec<usize>,
    effects: Vec<EffectInfo>,
}

#[derive(Debug, Clone)]
pub struct EffectInfo {
    name: String,
    index: usize,
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
        debug_counter: 0,
    }))
}

static RESOLVED_MIGRATIONS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<crate::KeyframeTrackParams>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

impl aviutl2_eframe::eframe::App for KeyframesGui {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                .call_read_section(|read| self.update_selected_object_info(read))
                .map_err(anyhow::Error::from)
                .flatten();
            if let Err(e) = update_selected_object_info {
                tracing::error!("Failed to update selected object info: {:?}", e);
            }
        }
    }
    fn ui(&mut self, ui: &mut aviutl2_eframe::egui::Ui, frame: &mut aviutl2_eframe::eframe::Frame) {
        ui.request_repaint_after(std::time::Duration::from_millis(100));
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if crate::EDIT_HANDLE.is_ready() {
                if self.is_undo_mode() {
                    self.render_undo_mode_warning(ui);
                }
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

    fn render_selected_object_info(&mut self, ui: &mut egui::Ui) {
        let Some(selected_object_info) = &self.selected_object_info else {
            ui.label("No object selected");
            return;
        };
        // ui.label(format!("Selected Object: {}", selected_object_info.name));
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new(format!("Selected Object: {}", selected_object_info.name))
                        .color(
                            if crate::module::DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed)
                            {
                                aviutl2::config::get_color_code("LogWarn")
                                    .expect("Null文字はない")
                                    .expect("そもそもこれが落ちるなら本体も落ちる")
                                    .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b))
                            } else {
                                ui.visuals().widgets.noninteractive.fg_stroke.color
                            },
                        ),
                )
                .sense(egui::Sense::click()),
            )
            .clicked()
        {
            self.debug_counter += 1;
            if self.debug_counter >= 5 {
                let debug_mode = self.debug_counter.is_multiple_of(2);
                tracing::info!("Setting debug mode to {}", debug_mode);
                crate::module::DEBUG_MODE.store(debug_mode, std::sync::atomic::Ordering::Relaxed);
            }
        }
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
        egui::containers::CollapsingHeader::new(format!("Effect: {}", effect.name))
            .id_salt((effect.index, &effect.name))
            .enabled(!effect.keyframe_tracks.is_empty())
            .open(effect.keyframe_tracks.is_empty().then_some(false))
            .show(ui, |ui| {
                for (params, track) in &effect.keyframe_tracks {
                    ui.push_id(&track.names, |ui| {
                        self.render_keyframe_track_info(ui, info, object, effect, params, track);
                    });
                }
            });
    }

    fn render_keyframe_track_info(
        &self,
        ui: &mut egui::Ui,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
    ) {
        ui.horizontal_wrapped(|ui| {
            for name in &track.names {
                ui.menu_button(name, |ui| {
                    if ui.button("分離").clicked() {
                        self.detach_keyframe_track(object, effect, params, track, name);
                    }
                });
            }
        });
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

        let total_frames = object.frames.last().unwrap() - object.frames.first().unwrap();
        self.render_track_background(
            &painter,
            response.rect,
            &current_object_color,
            num_divisions,
        );

        let Some(keyframes) = crate::KEYFRAMES.get(params) else {
            self.render_frame_cursor(&painter, info, object, response.rect, total_frames);
            return;
        };
        let sections = Self::track_sections(object, total_frames);
        if sections.len() != keyframes.keyframes.len() - 1 {
            return;
        }

        self.render_keyframe_section_interactions(
            ui,
            &painter,
            response.rect,
            object,
            effect,
            params,
            track,
            &keyframes,
            &sections,
            selected_object_color,
        );
        self.render_easing_labels(
            ui,
            &painter,
            response.rect,
            object,
            &keyframes,
            total_frames,
        );
        self.render_midpoint_lines(
            ui,
            &painter,
            response.rect,
            object,
            &keyframes,
            total_frames,
        );
        self.render_frame_cursor(&painter, info, object, response.rect, total_frames);
    }

    fn render_track_background(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        current_object_color: &[egui::Color32],
        num_divisions: usize,
    ) {
        let width_per_section = rect.width() / num_divisions as f32;
        for i in 0..num_divisions {
            let mut section_rect = rect;
            section_rect.set_left(rect.left() + i as f32 * width_per_section);
            section_rect.set_right((section_rect.left() + width_per_section).min(rect.right()));
            let position = i as f32 / num_divisions as f32;
            let color = current_object_color[position.floor() as usize].lerp_to_gamma(
                current_object_color
                    [(position.ceil() as usize).min(current_object_color.len() - 1)],
                position.fract(),
            );
            painter.rect_filled(section_rect, 0.0, color);
        }
    }

    fn track_sections(object: &SelectedObjectInfo, total_frames: usize) -> Vec<(usize, f32, f32)> {
        let mut sections = vec![];
        for i in 0..object.frames.len() - 1 {
            let left_position = (object.frames[i] - object.frames[0]) as f32 / total_frames as f32;
            let right_position =
                (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
            sections.push((i, left_position, right_position));
        }
        sections
    }

    fn render_keyframe_section_interactions(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
        keyframes: &crate::curve::Keyframes,
        sections: &[(usize, f32, f32)],
        selected_object_color: egui::Color32,
    ) {
        let crate::curve::Keyframe::Easing(ref initial_kf_info) = keyframes.keyframes[0] else {
            unreachable!();
        };
        let mut kf_info = initial_kf_info;

        for section in sections {
            if let crate::curve::Keyframe::Easing(ref new_kf_info) = keyframes.keyframes[section.0]
            {
                kf_info = new_kf_info;
            }

            let rect = Self::section_rect(track_rect, section.1, section.2);
            let response = ui
                .interact(
                    rect,
                    ui.id().with(section.0),
                    aviutl2_eframe::egui::Sense::click(),
                )
                .on_hover_text(Self::easing_hover_text(kf_info));
            if response.hovered() {
                painter.rect_filled(rect, 0.0, selected_object_color);
            }

            egui::containers::Popup::menu(&response).show(|ui| {
                self.show_easing_menu(ui, keyframes, params, section.0, "", |new_keyframes| {
                    self.update_track_keyframes(object, effect, track, section.0, new_keyframes);
                });
            });
        }
    }

    fn section_rect(track_rect: egui::Rect, left: f32, right: f32) -> egui::Rect {
        let mut rect = track_rect;
        rect.set_left(track_rect.left() + left * track_rect.width());
        rect.set_right(track_rect.left() + right * track_rect.width());
        rect
    }

    fn easing_hover_text(kf_info: &crate::curve::EasingKeyframeInfo) -> String {
        if kf_info.params.is_empty() {
            return kf_info.easing.clone();
        }

        format!(
            "{}：{}",
            kf_info.easing,
            kf_info
                .params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn update_track_keyframes(
        &self,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        track: &KeyframeTrackInfo,
        section_index: usize,
        new_keyframes: crate::curve::Keyframes,
    ) {
        tracing::info!(
            "Updating keyframe {:?} of track {:?} in effect {:?} to {:?}",
            section_index,
            track.names,
            effect.name,
            &new_keyframes
        );
        let new_params = crate::KeyframeTrackParams::new();
        crate::KEYFRAMES.insert(new_params, new_keyframes);
        let edit_result = crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                for name in &track.names {
                    let mut before = edit.get_object_effect_item(
                        object.handle,
                        &effect.name,
                        effect.index,
                        name,
                    )?;
                    new_params.set_params(&mut before)?;
                    edit.set_object_effect_item(
                        object.handle,
                        &effect.name,
                        effect.index,
                        name,
                        &before,
                    )?;
                }
                anyhow::Ok(())
            })
            .map_err(anyhow::Error::from)
            .flatten();
        match edit_result {
            Ok(()) => {
                tracing::info!(
                    "Updated keyframe track params for section {} of track {:?} in effect {:?} to {:?}",
                    section_index,
                    track.names,
                    effect.name,
                    new_params
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to update keyframe track params for section {} of track {:?} in effect {:?}: {:?}",
                    section_index,
                    track.names,
                    effect.name,
                    e
                );
            }
        }
    }

    fn render_easing_labels(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        object: &SelectedObjectInfo,
        keyframes: &crate::curve::Keyframes,
        total_frames: usize,
    ) {
        for (i, frame) in object.frames.iter().enumerate() {
            if i == object.frames.len() - 1 {
                continue;
            }
            let easing = match keyframes.keyframes[i] {
                crate::curve::Keyframe::Easing(ref easing) => easing.easing.as_str(),
                _ => "-",
            };
            let left_position = (*frame - object.frames[0]) as f32 / total_frames as f32;
            let right_position =
                (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
            let mut rect = Self::section_rect(track_rect, left_position, right_position);
            rect.set_left(rect.left() + ui.spacing().button_padding.x);

            let color = if matches!(keyframes.keyframes[i], crate::curve::Keyframe::Ignored) {
                ui.visuals()
                    .widgets
                    .noninteractive
                    .fg_stroke
                    .color
                    .linear_multiply(0.25)
            } else {
                ui.visuals().widgets.noninteractive.fg_stroke.color
            };
            let mut layout = egui::text::LayoutJob::default();
            layout.append(
                easing,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::default(),
                    color,
                    ..Default::default()
                },
            );
            layout.wrap = egui::text::TextWrapping::truncate_at_width(rect.width());
            let galley = painter.layout_job(layout);
            painter.galley(
                rect.left_center().tap_mut(|pos| {
                    pos.y -= galley.size().y / 2.0;
                }),
                galley,
                color,
            );
        }
    }

    fn render_midpoint_lines(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        track_rect: egui::Rect,
        object: &SelectedObjectInfo,
        keyframes: &crate::curve::Keyframes,
        total_frames: usize,
    ) {
        for (i, frame) in object.frames.iter().enumerate() {
            if i == 0 || i == object.frames.len() - 1 {
                continue;
            }
            let position = (*frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            let mut rect = track_rect;
            rect.set_left(rect.left() + position * track_rect.width() - 1.0);
            rect.set_right(rect.left() + 1.0);
            let color = if matches!(keyframes.keyframes[i], crate::curve::Keyframe::Ignored) {
                ui.visuals()
                    .widgets
                    .noninteractive
                    .bg_fill
                    .linear_multiply(0.25)
            } else {
                ui.visuals().widgets.noninteractive.bg_fill
            };
            painter.rect_filled(rect, 0.0, color);
        }
    }

    fn render_frame_cursor(
        &self,
        painter: &egui::Painter,
        info: &aviutl2::generic::EditInfo,
        object: &SelectedObjectInfo,
        track_rect: egui::Rect,
        total_frames: usize,
    ) {
        if *object.frames.first().unwrap() <= info.frame
            && info.frame <= *object.frames.last().unwrap()
        {
            let position =
                (info.frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
            let mut rect = track_rect;
            rect.set_left(rect.left() + position * track_rect.width() - 1.0);
            rect.set_right(rect.left() + 1.0);
            let selected_line = aviutl2::config::get_color_code("FrameCursor")
                .expect("Null文字はない")
                .expect("そもそもこれが落ちるなら本体も落ちる")
                .pipe(|(r, g, b)| egui::Color32::from_rgb(r, g, b));
            painter.rect_filled(rect, 0.0, selected_line);
        }
    }

    fn detach_keyframe_track(
        &self,
        object: &SelectedObjectInfo,
        effect: &EffectInfo,
        params: &crate::KeyframeTrackParams,
        track: &KeyframeTrackInfo,
        name: &str,
    ) {
        let res = crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                let new_params = crate::KeyframeTrackParams::new();
                if let Some(keyframes) = crate::KEYFRAMES.get(params) {
                    crate::KEYFRAMES.insert(new_params, keyframes.clone());
                }
                let mut before =
                    edit.get_object_effect_item(object.handle, &effect.name, effect.index, name)?;
                new_params.set_params(&mut before)?;
                edit.set_object_effect_item(
                    object.handle,
                    &effect.name,
                    effect.index,
                    name,
                    &before,
                )?;
                anyhow::Ok(())
            })
            .map_err(anyhow::Error::from)
            .flatten();
        match res {
            Ok(()) => {
                tracing::info!(
                    "Detached keyframe track {:?} of effect {:?} in object {:?}",
                    track.names,
                    effect.name,
                    object.name
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to detach keyframe track {:?} of effect {:?} in object {:?}: {:?}",
                    track.names,
                    effect.name,
                    object.name,
                    e
                );
            }
        }
    }

    fn show_easing_menu(
        &self,
        ui: &mut egui::Ui,
        keyframes: &crate::curve::Keyframes,
        _params: &crate::KeyframeTrackParams,
        index: usize,
        current_level: &str,
        update_keyframe: impl FnOnce(crate::curve::Keyframes),
    ) {
        let default = indexmap::IndexMap::new();
        let easings = crate::EASINGS.get().unwrap_or(&default);
        let mut update_keyframe_once = Some(update_keyframe);
        let mut update_keyframe = |new_keyframes: crate::curve::Keyframes| {
            if let Some(f) = update_keyframe_once.take() {
                f(new_keyframes);
            }
        };

        let (keyframe_index, current_keyframe) = &keyframes
            .keyframes
            .iter()
            .enumerate()
            .take(index + 1)
            .rfind(|(_, k)| matches!(k, crate::curve::Keyframe::Easing(_)))
            .expect(
                "少なくとも0フレーム目にはイージングが設定されているはずなので、必ず見つかるはず",
            );
        let keyframe_index = *keyframe_index;
        let crate::curve::Keyframe::Easing(current_keyframe) = current_keyframe else {
            unreachable!();
        };
        let current_easing = easings.get(&current_keyframe.easing);

        // TODO: ちゃんとlabelごとに階層にする
        egui::ScrollArea::vertical().show(ui, |ui| {
            Self::show_midpoint_actions(ui, keyframes, index, current_level, &mut update_keyframe);
            if let Some(current_easing) = current_easing {
                Self::show_current_easing_options(
                    ui,
                    keyframes,
                    keyframe_index,
                    current_keyframe,
                    current_easing,
                    index,
                    current_level,
                    &mut update_keyframe,
                );
            }
            Self::show_easing_choices(ui, keyframes, index, easings, &mut update_keyframe);
        });
    }

    fn show_midpoint_actions(
        ui: &mut egui::Ui,
        keyframes: &crate::curve::Keyframes,
        index: usize,
        current_level: &str,
        update_keyframe: &mut impl FnMut(crate::curve::Keyframes),
    ) {
        if !current_level.is_empty() || index == 0 {
            return;
        }

        if ui.button("引き継ぎ").clicked() {
            let mut new_keyframes = keyframes.clone();
            new_keyframes.keyframes[index] = crate::curve::Keyframe::Midpoint;
            update_keyframe(new_keyframes);
        }
        if ui.button("無視").clicked() {
            let mut new_keyframes = keyframes.clone();
            new_keyframes.keyframes[index] = crate::curve::Keyframe::Ignored;
            update_keyframe(new_keyframes);
        }
        ui.separator();
    }

    fn show_current_easing_options(
        ui: &mut egui::Ui,
        keyframes: &crate::curve::Keyframes,
        keyframe_index: usize,
        current_keyframe: &crate::curve::EasingKeyframeInfo,
        current_easing: &crate::curve::Easing,
        index: usize,
        current_level: &str,
        update_keyframe: &mut impl FnMut(crate::curve::Keyframes),
    ) {
        if current_easing.has_speed {
            Self::show_speed_options(
                ui,
                keyframes,
                keyframe_index,
                current_keyframe,
                update_keyframe,
            );
        }
        if current_easing.has_timecontrol && ui.button("時間制御").clicked() {
            tracing::warn!(
                "Unimplemented: Opening time control dialog for section {} of track {:?} in effect {:?}",
                index,
                current_easing.name,
                current_level
            );
        }
        ui.separator();
    }

    fn show_speed_options(
        ui: &mut egui::Ui,
        keyframes: &crate::curve::Keyframes,
        keyframe_index: usize,
        current_keyframe: &crate::curve::EasingKeyframeInfo,
        update_keyframe: &mut impl FnMut(crate::curve::Keyframes),
    ) {
        let mut current_acceleration = current_keyframe.acceleration;
        if ui.checkbox(&mut current_acceleration, "加速").changed() {
            let mut new_keyframes = keyframes.clone();
            let crate::curve::Keyframe::Easing(ref mut k) = new_keyframes.keyframes[keyframe_index]
            else {
                unreachable!();
            };
            k.acceleration = current_acceleration;
            update_keyframe(new_keyframes);
        }

        let mut current_deceleration = current_keyframe.deceleration;
        if ui.checkbox(&mut current_deceleration, "減速").changed() {
            let mut new_keyframes = keyframes.clone();
            let crate::curve::Keyframe::Easing(ref mut k) = new_keyframes.keyframes[keyframe_index]
            else {
                unreachable!();
            };
            k.deceleration = current_deceleration;
            update_keyframe(new_keyframes);
        }
    }

    fn show_easing_choices(
        ui: &mut egui::Ui,
        keyframes: &crate::curve::Keyframes,
        index: usize,
        easings: &indexmap::IndexMap<String, crate::curve::Easing>,
        update_keyframe: &mut impl FnMut(crate::curve::Keyframes),
    ) {
        for easing in easings.values() {
            if ui.button(&easing.name).clicked() {
                let new_keyframes = Self::keyframes_with_easing(keyframes, index, easing);
                update_keyframe(new_keyframes);
            }
        }
    }

    fn keyframes_with_easing(
        keyframes: &crate::curve::Keyframes,
        index: usize,
        easing: &crate::curve::Easing,
    ) -> crate::curve::Keyframes {
        let mut new_keyframes = keyframes.clone();
        new_keyframes.keyframes[index] =
            crate::curve::Keyframe::Easing(crate::curve::EasingKeyframeInfo {
                easing: easing.name.clone(),
                acceleration: easing.default_acceleration,
                deceleration: easing.default_deceleration,
                params: easing.params.values().cloned().collect(),
            });

        if easing.ignore_midpoints {
            Self::ignore_following_midpoints(&mut new_keyframes, index);
        }
        new_keyframes
    }

    fn ignore_following_midpoints(keyframes: &mut crate::curve::Keyframes, index: usize) {
        for i in index + 1..keyframes.keyframes.len() {
            if !matches!(keyframes.keyframes[i], crate::curve::Keyframe::Midpoint) {
                break;
            }
            keyframes.keyframes[i] = crate::curve::Keyframe::Ignored;
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
