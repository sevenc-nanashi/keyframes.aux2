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
        for (params, bindings) in &bindings {
            for binding in bindings {
                let effect_key = (
                    binding.object,
                    binding.effect_name.clone(),
                    binding.effect_index,
                );
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
                egui::Label::new(format!("Selected Object: {}", selected_object_info.name))
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
        ui.collapsing(format!("Effect: {}", effect.name), |ui| {
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

        let total_frames = object.frames.last().unwrap() - object.frames.first().unwrap();
        // 背景（グラデーション）
        let width_per_section = response.rect.width() / num_divisions as f32;
        for i in 0..num_divisions {
            let mut rect = response.rect;
            rect.set_left(rect.left() + i as f32 * width_per_section);
            rect.set_right((rect.left() + width_per_section).min(response.rect.right()));
            let position = i as f32 / num_divisions as f32;
            let color = current_object_color[position.floor() as usize].lerp_to_gamma(
                current_object_color
                    [(position.ceil() as usize).min(current_object_color.len() - 1)],
                position.fract(),
            );
            painter.rect_filled(rect, 0.0, color);
        }

        if let Some(keyframes) = crate::KEYFRAMES.get(params) {
            let mut sections = vec![];
            for i in 0..object.frames.len() - 1 {
                let left_position =
                    (object.frames[i] - object.frames[0]) as f32 / total_frames as f32;
                let right_position =
                    (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
                sections.push((i, left_position, right_position));
            }
            // ホバーしているセクションの強調表示
            for section in sections {
                let mut rect = response.rect;
                let left_position = response.rect.left() + section.1 * response.rect.width();
                let right_position = response.rect.left() + section.2 * response.rect.width();
                rect.set_left(left_position);
                rect.set_right(right_position);
                let response = ui.interact(
                    rect,
                    ui.id().with(section.0),
                    aviutl2_eframe::egui::Sense::click(),
                );
                if response.hovered() {
                    painter.rect_filled(rect, 0.0, selected_object_color);
                }
                egui::containers::Popup::menu(&response).show(|ui| {
                    self.show_easing_menu(ui, &keyframes, params, section.0, "", |new_keyframes| {
                        tracing::info!(
                            "Updating keyframe {:?} of track {:?} in effect {:?} to {:?}",
                            section.0,
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
                                    section.0,
                                    track.names,
                                    effect.name,
                                    new_params
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to update keyframe track params for section {} of track {:?} in effect {:?}: {:?}",
                                    section.0,
                                    track.names,
                                    effect.name,
                                    e
                                );
                            }
                            }
                    });
                });
            }
            // if let Some(hovered_section) = hovered_section {
            //     let mut rect = response.rect;
            //     let left_position =
            //         response.rect.left() + hovered_section.1 * response.rect.width();
            //     let right_position =
            //         response.rect.left() + hovered_section.2 * response.rect.width();
            //     rect.set_left(left_position);
            //     rect.set_right(right_position);
            //     painter.rect_filled(rect, 0.0, selected_object_color);
            // }

            // 現在のイージング
            for (i, frame) in object.frames.iter().enumerate() {
                if i == object.frames.len() - 1 {
                    continue;
                }
                let easing = match keyframes.keyframes[i] {
                    crate::curve::Keyframe::Easing(ref easing) => Some(easing.easing.as_str()),
                    _ => Some("-"),
                };
                let left_position = (*frame - object.frames[0]) as f32 / total_frames as f32;
                let right_position =
                    (object.frames[i + 1] - object.frames[0]) as f32 / total_frames as f32;
                let mut rect = response.rect;
                rect.set_left(rect.left() + left_position * response.rect.width());
                rect.set_right(
                    rect.left() + (right_position - left_position) * response.rect.width(),
                );
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
                    if let Some(easing) = easing {
                        easing
                    } else {
                        "-"
                    },
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

            // 中間点の線
            for (i, frame) in object.frames.iter().enumerate() {
                if i == 0 || i == object.frames.len() - 1 {
                    continue;
                }
                let position =
                    (*frame - object.frames.first().unwrap()) as f32 / total_frames as f32;
                let mut rect = response.rect;
                rect.set_left(rect.left() + position * response.rect.width() - 1.0);
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
        // カーソル
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
            if current_level.is_empty() && index > 0 {
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
            if let Some(current_easing) = current_easing {
                if current_easing.has_speed {
                    let mut current_acceleration = current_keyframe.acceleration;
                    if ui.checkbox(&mut current_acceleration, "加速").changed() {
                        let mut new_keyframes = keyframes.clone();
                        let crate::curve::Keyframe::Easing(ref mut k) = new_keyframes.keyframes[keyframe_index] else {
                            unreachable!();
                        };
                        k.acceleration = current_acceleration;
                        update_keyframe(new_keyframes);
                    }
                    let mut current_deceleration = current_keyframe.deceleration;
                    if ui.checkbox(&mut current_deceleration, "減速").changed() {
                        let mut new_keyframes = keyframes.clone();
                        let crate::curve::Keyframe::Easing(ref mut k) = new_keyframes.keyframes[keyframe_index] else {
                            unreachable!();
                        };
                        k.deceleration = current_deceleration;
                        update_keyframe(new_keyframes);
                    }
                }
                if current_easing.has_timecontrol
                    && ui.button("時間制御").clicked() {
                        tracing::warn!("Unimplemented: Opening time control dialog for section {} of track {:?} in effect {:?}",
                            index,
                            current_easing.name,
                            current_level
                        );
                }
                ui.separator();
            }
            for easing in easings.values() {
                if ui.button(&easing.name).clicked() {
                    let mut new_keyframes = keyframes.clone();
                    new_keyframes.keyframes[index] = crate::curve::Keyframe::Easing(crate::curve::EasingKeyframeInfo {
                        easing: easing.name.clone(),
                        acceleration: easing.default_acceleration,
                        deceleration: easing.default_deceleration,
                        params: easing.params.values().cloned().collect(),
                    });
                    if easing.ignore_midpoints {
                        for i in index + 1..keyframes.keyframes.len() {
                            if matches!(new_keyframes.keyframes[i], crate::curve::Keyframe::Midpoint) {
                                new_keyframes.keyframes[i] = crate::curve::Keyframe::Ignored;
                            } else {
                                break;
                            }
                        }
                    }
                    update_keyframe(new_keyframes);
                }
            }
        });
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
