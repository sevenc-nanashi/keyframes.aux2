use super::*;
use anyhow::Context;
use aviutl2_eframe::egui;

impl KeyframesGui {
    fn update_track_keyframes_by_target(
        target: &TimeControlEditorTarget,
        new_keyframes: crate::curve::Keyframes,
    ) -> Option<crate::KeyframeTrackParams> {
        tracing::info!(
            "Updating time control keyframe {:?} of track {:?} in effect {:?} to {:?}",
            target.keyframe_index,
            target.track_names,
            target.effect_name,
            &new_keyframes
        );
        let new_params = crate::KeyframeTrackParams::new();
        crate::KEYFRAMES.insert(new_params, new_keyframes);
        let edit_result = crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                for name in &target.track_names {
                    let mut before = edit.get_object_effect_item(
                        target.object,
                        &target.effect_name,
                        target.effect_index,
                        name,
                    )?;
                    new_params.set_params(&mut before)?;
                    edit.set_object_effect_item(
                        target.object,
                        &target.effect_name,
                        target.effect_index,
                        name,
                        &before,
                    )?;
                }
                anyhow::Ok(())
            })
            .map_err(anyhow::Error::from)
            .flatten();
        match edit_result {
            Ok(()) => Some(new_params),
            Err(e) => {
                tracing::error!(
                    "Failed to update time control keyframe {:?} of track {:?} in effect {:?}: {:?}",
                    target.keyframe_index,
                    target.track_names,
                    target.effect_name,
                    e
                );
                None
            }
        }
    }

    pub(super) fn render_timecontrol_editor(&mut self, ui: &mut egui::Ui) {
        let Some(mut target) = self.timecontrol_editor.clone() else {
            return;
        };
        let keyframes = match crate::KEYFRAMES.get(&target.params) {
            Some(keyframes) => keyframes,
            None => {
                self.timecontrol_editor = None;
                return;
            }
        };
        let keyframe = match keyframes.keyframes.get(target.keyframe_index) {
            Some(crate::curve::Keyframe::Easing(kf_info)) => kf_info,
            _ => {
                self.timecontrol_editor = None;
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if ui.button("戻る").clicked() {
                self.timecontrol_editor = None;
            }
            ui.label(format!(
                "時間制御：{} / {} / {}",
                target.effect_name,
                if target.track_names.len() == 1 {
                    target.track_names[0].clone()
                } else {
                    format!(
                        "{} + {}",
                        target.track_names[0],
                        target.track_names.len() - 1
                    )
                },
                keyframe.easing
            ));
        });
        ui.separator();

        if self.timecontrol_editor.is_none() {
            return;
        }

        let (changed, commit_requested) = Self::show_timecontrol_bezier_editor(
            ui,
            &mut target.timecontrol,
            &mut target.selected_point,
            &mut target.context_menu_position,
        );
        target.dirty |= changed;

        if commit_requested && target.dirty {
            let Some(mut new_keyframes) = crate::KEYFRAMES
                .get(&target.params)
                .map(|keyframes| keyframes.clone())
            else {
                self.timecontrol_editor = None;
                return;
            };
            let Some(crate::curve::Keyframe::Easing(kf_info)) =
                new_keyframes.keyframes.get_mut(target.keyframe_index)
            else {
                self.timecontrol_editor = None;
                return;
            };
            kf_info.timecontrol = target.timecontrol.clone();
            if let Some(new_params) = Self::update_track_keyframes_by_target(&target, new_keyframes)
            {
                target.params = new_params;
                target.dirty = false;
            }
        }

        self.timecontrol_editor = Some(target);
    }

    fn show_timecontrol_bezier_editor(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::curve::TimeControlBezier,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
    ) -> (bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;
        let mut structure_changed = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        let available_size = ui.available_size();
        let size = available_size.x.min(available_size.y).max(220.0);
        let (response, painter) =
            ui.allocate_painter(egui::Vec2::splat(size), egui::Sense::click());
        let rect = response.rect.shrink(size * 0.15);
        let (min_y, max_y) = (0.0, 1.0);

        let to_screen = |point: [f64; 2]| {
            egui::pos2(
                rect.left() + point[0] as f32 * rect.width(),
                rect.bottom() - ((point[1] - min_y) / (max_y - min_y)) as f32 * rect.height(),
            )
        };
        let from_screen = |point: egui::Pos2| {
            [
                ((point.x - rect.left()) / rect.width()) as f64,
                min_y + ((rect.bottom() - point.y) / rect.height()) as f64 * (max_y - min_y),
            ]
        };
        if response.secondary_clicked() {
            *context_menu_position = response.interact_pointer_pos().map(from_screen);
        }
        response.context_menu(|ui| {
            if ui.button("中継点追加").clicked() {
                *selected_point = Self::insert_timecontrol_point(
                    timecontrol,
                    context_menu_position.unwrap_or([0.5, 0.5]),
                );
                changed = true;
                commit_requested = true;
                ui.close();
            }
        });

        let grid_stroke = egui::Stroke::new(1.0, GUI_COLORS.grid_line);
        let graph_rect = egui::Rect::from_min_max(to_screen([0.0, 1.0]), to_screen([1.0, 0.0]));
        painter.rect_stroke(graph_rect, 0.0, grid_stroke, egui::StrokeKind::Inside);
        for i in 1..4 {
            let t = i as f32 / 4.0;
            painter.line_segment(
                [
                    egui::pos2(graph_rect.left() + graph_rect.width() * t, graph_rect.top()),
                    egui::pos2(
                        graph_rect.left() + graph_rect.width() * t,
                        graph_rect.bottom(),
                    ),
                ],
                grid_stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(
                        graph_rect.left(),
                        graph_rect.top() + graph_rect.height() * t,
                    ),
                    egui::pos2(
                        graph_rect.right(),
                        graph_rect.top() + graph_rect.height() * t,
                    ),
                ],
                grid_stroke,
            );
        }

        let curve_color = GUI_COLORS.zoom_gauge;
        let curve_stroke = egui::Stroke::new(2.0, curve_color);
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let mut previous = to_screen(timecontrol.points[segment_index].position);
            for i in 1..=24 {
                let t = i as f64 / 24.0;
                let point = timecontrol.segment_point_at(segment_index, t);
                let current = to_screen(point);
                painter.line_segment([previous, current], curve_stroke);
                previous = current;
            }
        }

        let control_stroke = egui::Stroke::new(1.0, GUI_COLORS.anchor_line);
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let start = to_screen(timecontrol.points[segment_index].position);
            let end = to_screen(timecontrol.points[segment_index + 1].position);
            let out_handle = to_screen(
                timecontrol.points[segment_index]
                    .out_handle
                    .unwrap_or(timecontrol.points[segment_index].position),
            );
            let in_handle = to_screen(
                timecontrol.points[segment_index + 1]
                    .in_handle
                    .unwrap_or(timecontrol.points[segment_index + 1].position),
            );
            painter.line_segment([start, out_handle], control_stroke);
            painter.line_segment([in_handle, end], control_stroke);
        }

        for point_index in 0..timecontrol.points.len() {
            for handle_kind in [TimeControlHandleKind::In, TimeControlHandleKind::Out] {
                let Some(handle) = (match handle_kind {
                    TimeControlHandleKind::In => timecontrol.points[point_index].in_handle,
                    TimeControlHandleKind::Out => timecontrol.points[point_index].out_handle,
                }) else {
                    continue;
                };

                let handle_pos = to_screen(handle);
                let handle_rect = egui::Rect::from_center_size(handle_pos, egui::Vec2::splat(18.0));
                let handle_response = ui.interact(
                    handle_rect,
                    ui.id()
                        .with(("timecontrol_handle", point_index, handle_kind.id())),
                    egui::Sense::click_and_drag(),
                );
                if handle_response.clicked() || handle_response.dragged() {
                    *selected_point = point_index;
                }
                if handle_response.secondary_clicked() {
                    *selected_point = point_index;
                    *context_menu_position =
                        handle_response.interact_pointer_pos().map(from_screen);
                }
                if handle_response.dragged()
                    && let Some(pointer_pos) = handle_response.interact_pointer_pos()
                {
                    let new_point = Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        handle_kind,
                        from_screen(pointer_pos),
                    );
                    let changed_handle = match handle_kind {
                        TimeControlHandleKind::In => {
                            timecontrol.points[point_index].in_handle != Some(new_point)
                        }
                        TimeControlHandleKind::Out => {
                            timecontrol.points[point_index].out_handle != Some(new_point)
                        }
                    };
                    if changed_handle {
                        match handle_kind {
                            TimeControlHandleKind::In => {
                                timecontrol.points[point_index].in_handle = Some(new_point);
                                if !timecontrol.points[point_index].handles_separated {
                                    Self::mirror_timecontrol_handle(
                                        timecontrol,
                                        point_index,
                                        false,
                                    );
                                }
                            }
                            TimeControlHandleKind::Out => {
                                timecontrol.points[point_index].out_handle = Some(new_point);
                                if !timecontrol.points[point_index].handles_separated {
                                    Self::mirror_timecontrol_handle(timecontrol, point_index, true);
                                }
                            }
                        }
                        changed = true;
                    }
                }
                commit_requested |= handle_response.drag_stopped();
                let color = if handle_response.hovered() || handle_response.dragged() {
                    GUI_COLORS.anchor_hover
                } else {
                    GUI_COLORS.anchor
                };
                painter.circle_filled(handle_pos, 4.0, color.linear_multiply(0.85));
                handle_response.context_menu(|ui| {
                    if ui.button("中継点追加").clicked() {
                        *selected_point = Self::insert_timecontrol_point(
                            timecontrol,
                            context_menu_position.unwrap_or_else(|| from_screen(handle_pos)),
                        );
                        changed = true;
                        commit_requested = true;
                        structure_changed = true;
                        ui.close();
                    }
                    ui.separator();
                    if Self::show_timecontrol_handle_menu(ui, timecontrol, selected_point) {
                        changed = true;
                        commit_requested = true;
                    }
                });
                if structure_changed {
                    return (changed, commit_requested);
                }
            }
        }

        for point_index in 0..timecontrol.points.len() {
            let point = to_screen(timecontrol.points[point_index].position);
            let handle_rect = egui::Rect::from_center_size(point, egui::Vec2::splat(18.0));
            let handle_response = ui.interact(
                handle_rect,
                ui.id().with(("timecontrol_anchor", point_index)),
                egui::Sense::click_and_drag(),
            );
            if handle_response.clicked() || handle_response.dragged() {
                *selected_point = point_index;
            }
            if handle_response.secondary_clicked() {
                *selected_point = point_index;
                *context_menu_position = handle_response.interact_pointer_pos().map(from_screen);
            }
            if handle_response.dragged()
                && let Some(pointer_pos) = handle_response.interact_pointer_pos()
                && Self::move_timecontrol_anchor(timecontrol, point_index, from_screen(pointer_pos))
            {
                changed = true;
            }
            commit_requested |= handle_response.drag_stopped();
            let color = if handle_response.hovered()
                || handle_response.dragged()
                || point_index == *selected_point
            {
                GUI_COLORS.anchor_select
            } else {
                GUI_COLORS.anchor
            };
            // painter.circle_filled(point, 5.0, color);
            painter.rect_filled(
                egui::Rect::from_center_size(point, egui::Vec2::splat(10.0)),
                2.0,
                color,
            );
            let before_len = timecontrol.points.len();
            handle_response.context_menu(|ui| {
                if Self::show_timecontrol_anchor_menu(
                    ui,
                    timecontrol,
                    selected_point,
                    context_menu_position.unwrap_or_else(|| from_screen(point)),
                ) {
                    changed = true;
                    commit_requested = true;
                }
            });
            if timecontrol.points.len() != before_len {
                return (changed, commit_requested);
            }
        }

        (changed, commit_requested)
    }

    fn show_timecontrol_anchor_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::curve::TimeControlBezier,
        selected_point: &mut usize,
        add_point_position: [f64; 2],
    ) -> bool {
        let mut changed = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        if ui.button("中継点追加").clicked() {
            *selected_point = Self::insert_timecontrol_point(timecontrol, add_point_position);
            changed = true;
            ui.close();
        }

        let can_remove = *selected_point != 0 && *selected_point + 1 < timecontrol.points.len();
        if ui
            .add_enabled(can_remove, egui::Button::new("中継点削除"))
            .clicked()
        {
            Self::remove_timecontrol_point(timecontrol, selected_point);
            changed = true;
            ui.close();
        }

        ui.separator();
        changed |= Self::show_timecontrol_handle_menu(ui, timecontrol, selected_point);

        changed
    }

    fn insert_timecontrol_point(
        timecontrol: &mut crate::curve::TimeControlBezier,
        position: [f64; 2],
    ) -> usize {
        let x = position[0].clamp(0.0, 1.0);
        let after_index = timecontrol
            .points
            .windows(2)
            .position(|points| x <= points[1].position[0])
            .unwrap_or(timecontrol.points.len().saturating_sub(2));
        let new_index = timecontrol.insert_midpoint(after_index);
        timecontrol.points[new_index].position = [
            x.clamp(
                timecontrol.points[new_index - 1].position[0]
                    + Self::TIMECONTROL_MIN_ANCHOR_DISTANCE,
                timecontrol.points[new_index + 1].position[0]
                    - Self::TIMECONTROL_MIN_ANCHOR_DISTANCE,
            ),
            position[1],
        ];
        Self::reset_timecontrol_handles(timecontrol, new_index);
        Self::constrain_all_timecontrol_handles(timecontrol);
        new_index
    }

    fn remove_timecontrol_point(
        timecontrol: &mut crate::curve::TimeControlBezier,
        selected_point: &mut usize,
    ) {
        let remove_index = *selected_point;
        timecontrol.remove_midpoint(remove_index);
        *selected_point = remove_index
            .saturating_sub(1)
            .min(timecontrol.points.len().saturating_sub(1));
        Self::constrain_all_timecontrol_handles(timecontrol);
    }

    fn show_timecontrol_handle_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::curve::TimeControlBezier,
        selected_point: &mut usize,
    ) -> bool {
        let mut changed = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        let has_both_handles = timecontrol.points[*selected_point].in_handle.is_some()
            && timecontrol.points[*selected_point].out_handle.is_some();
        let label = if timecontrol.points[*selected_point].handles_separated {
            "ハンドル連動"
        } else {
            "ハンドル分離"
        };
        if ui
            .add_enabled(has_both_handles, egui::Button::new(label))
            .clicked()
        {
            timecontrol.points[*selected_point].handles_separated =
                !timecontrol.points[*selected_point].handles_separated;
            if !timecontrol.points[*selected_point].handles_separated {
                Self::mirror_timecontrol_handle(timecontrol, *selected_point, true);
            }
            changed = true;
            ui.close();
        }

        let has_any_handle = timecontrol.points[*selected_point].in_handle.is_some()
            || timecontrol.points[*selected_point].out_handle.is_some();
        if ui
            .add_enabled(has_any_handle, egui::Button::new("ハンドルリセット"))
            .clicked()
        {
            Self::reset_timecontrol_handles(timecontrol, *selected_point);
            changed = true;
            ui.close();
        }

        changed
    }

    fn clamped_timecontrol_anchor_position(
        timecontrol: &crate::curve::TimeControlBezier,
        point_index: usize,
        position: [f64; 2],
    ) -> [f64; 2] {
        if point_index == 0 {
            return [0.0, 0.0];
        }
        if point_index + 1 == timecontrol.points.len() {
            return [1.0, 1.0];
        }

        let min_x = point_index as f64 * Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
        let max_x = 1.0
            - (timecontrol.points.len() - 1 - point_index) as f64
                * Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
        [position[0].clamp(min_x, max_x), position[1]]
    }

    const TIMECONTROL_MIN_ANCHOR_DISTANCE: f64 = 0.001;

    fn move_timecontrol_anchor(
        timecontrol: &mut crate::curve::TimeControlBezier,
        point_index: usize,
        position: [f64; 2],
    ) -> bool {
        let new_position =
            Self::clamped_timecontrol_anchor_position(timecontrol, point_index, position);
        if timecontrol.points[point_index].position == new_position {
            return false;
        }

        let old_position = timecontrol.points[point_index].position;
        let delta = [
            new_position[0] - old_position[0],
            new_position[1] - old_position[1],
        ];

        timecontrol.points[point_index].position = new_position;

        for index in (1..point_index).rev() {
            let max_x =
                timecontrol.points[index + 1].position[0] - Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
            if timecontrol.points[index].position[0] <= max_x {
                break;
            }
            timecontrol.points[index].position[0] = max_x;
        }

        for index in point_index + 1..timecontrol.points.len().saturating_sub(1) {
            let min_x =
                timecontrol.points[index - 1].position[0] + Self::TIMECONTROL_MIN_ANCHOR_DISTANCE;
            if timecontrol.points[index].position[0] >= min_x {
                break;
            }
            timecontrol.points[index].position[0] = min_x;
        }

        if let Some(in_handle) = timecontrol.points[point_index].in_handle {
            timecontrol.points[point_index].in_handle =
                Some(Self::constrain_timecontrol_handle_position(
                    timecontrol,
                    point_index,
                    TimeControlHandleKind::In,
                    [in_handle[0] + delta[0], in_handle[1] + delta[1]],
                ));
        }
        if let Some(out_handle) = timecontrol.points[point_index].out_handle {
            timecontrol.points[point_index].out_handle =
                Some(Self::constrain_timecontrol_handle_position(
                    timecontrol,
                    point_index,
                    TimeControlHandleKind::Out,
                    [out_handle[0] + delta[0], out_handle[1] + delta[1]],
                ));
        }
        Self::constrain_all_timecontrol_handles(timecontrol);
        true
    }

    fn constrain_all_timecontrol_handles(timecontrol: &mut crate::curve::TimeControlBezier) {
        for point_index in 0..timecontrol.points.len() {
            if let Some(in_handle) = timecontrol.points[point_index].in_handle {
                timecontrol.points[point_index].in_handle =
                    Some(Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        TimeControlHandleKind::In,
                        in_handle,
                    ));
            }
            if let Some(out_handle) = timecontrol.points[point_index].out_handle {
                timecontrol.points[point_index].out_handle =
                    Some(Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        TimeControlHandleKind::Out,
                        out_handle,
                    ));
            }
        }
    }

    fn constrain_timecontrol_handle_position(
        timecontrol: &crate::curve::TimeControlBezier,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        handle: [f64; 2],
    ) -> [f64; 2] {
        let anchor = timecontrol.points[point_index].position;
        let x =
            Self::clamped_timecontrol_handle_x(timecontrol, point_index, handle_kind, handle[0]);
        if (x - handle[0]).abs() < f64::EPSILON {
            return handle;
        }

        let delta = [handle[0] - anchor[0], handle[1] - anchor[1]];
        if (x - anchor[0]).abs() < f64::EPSILON {
            return [x, handle[1]];
        }
        if delta[0].abs() < f64::EPSILON {
            return [x, handle[1]];
        }

        let scale = (x - anchor[0]) / delta[0];
        if !scale.is_finite() {
            return [x, handle[1]];
        }

        [x, anchor[1] + delta[1] * scale]
    }

    fn clamped_timecontrol_handle_x(
        timecontrol: &crate::curve::TimeControlBezier,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        x: f64,
    ) -> f64 {
        match handle_kind {
            TimeControlHandleKind::In => {
                let min_x = point_index
                    .checked_sub(1)
                    .map(|index| timecontrol.points[index].position[0])
                    .unwrap_or(timecontrol.points[point_index].position[0]);
                Self::clamp_ordered(x, min_x, timecontrol.points[point_index].position[0])
            }
            TimeControlHandleKind::Out => {
                let max_x = timecontrol
                    .points
                    .get(point_index + 1)
                    .map(|point| point.position[0])
                    .unwrap_or(timecontrol.points[point_index].position[0]);
                Self::clamp_ordered(x, timecontrol.points[point_index].position[0], max_x)
            }
        }
    }

    fn clamp_ordered(value: f64, min: f64, max: f64) -> f64 {
        if min <= max {
            value.clamp(min, max)
        } else {
            value.clamp(max, min)
        }
    }

    fn mirror_timecontrol_handle(
        timecontrol: &mut crate::curve::TimeControlBezier,
        point_index: usize,
        moved_out_handle: bool,
    ) {
        if timecontrol.points[point_index].handles_separated {
            return;
        }
        let position = timecontrol.points[point_index].position;
        if moved_out_handle {
            let Some(out_handle) = timecontrol.points[point_index].out_handle else {
                return;
            };
            let new_in_handle = Self::constrain_timecontrol_handle_position(
                timecontrol,
                point_index,
                TimeControlHandleKind::In,
                [
                    position[0] * 2.0 - out_handle[0],
                    position[1] * 2.0 - out_handle[1],
                ],
            );
            if let Some(in_handle) = &mut timecontrol.points[point_index].in_handle {
                *in_handle = new_in_handle;
            }
        } else {
            let Some(in_handle) = timecontrol.points[point_index].in_handle else {
                return;
            };
            let new_out_handle = Self::constrain_timecontrol_handle_position(
                timecontrol,
                point_index,
                TimeControlHandleKind::Out,
                [
                    position[0] * 2.0 - in_handle[0],
                    position[1] * 2.0 - in_handle[1],
                ],
            );
            if let Some(out_handle) = &mut timecontrol.points[point_index].out_handle {
                *out_handle = new_out_handle;
            }
        }
    }

    fn reset_timecontrol_handles(
        timecontrol: &mut crate::curve::TimeControlBezier,
        point_index: usize,
    ) {
        let position = timecontrol.points[point_index].position;
        timecontrol.points[point_index].in_handle = point_index.checked_sub(1).map(|prev_index| {
            let prev = timecontrol.points[prev_index].position;
            [
                position[0] + (prev[0] - position[0]) / 3.0,
                position[1] + (prev[1] - position[1]) / 3.0,
            ]
        });
        timecontrol.points[point_index].out_handle =
            timecontrol.points.get(point_index + 1).map(|next_point| {
                let next = next_point.position;
                [
                    position[0] + (next[0] - position[0]) / 3.0,
                    position[1] + (next[1] - position[1]) / 3.0,
                ]
            });
        timecontrol.points[point_index].handles_separated = false;
    }

    pub(super) fn update_timecontrol_editor_target(
        &mut self,
        read: &aviutl2::generic::ReadSection,
    ) -> aviutl2::common::AnyResult<()> {
        let Some(target) = &self.timecontrol_editor else {
            return Ok(());
        };
        if target.dirty {
            return Ok(());
        }
        let track = read
            .get_object_effect_item(
                target.object,
                &target.effect_name,
                target.effect_index,
                &target.track_names[0],
            )
            .context("Failed to get object effect item for time control editor")?;
        let params = match crate::KeyframeTrackParams::parse(&track) {
            Some(params) => params,
            None => {
                tracing::error!(
                    "Failed to parse keyframe track params for time control editor, closing editor"
                );
                self.timecontrol_editor = None;
                return Ok(());
            }
        };
        let keyframes = crate::KEYFRAMES
            .get(&params)
            .context("Failed to get keyframes for time control editor")?
            .clone();
        self.timecontrol_editor.as_mut().unwrap().timecontrol =
            match keyframes.keyframes[target.keyframe_index] {
                crate::curve::Keyframe::Easing(ref easing) => easing.timecontrol.clone(),
                _ => anyhow::bail!("Target keyframe is not easing"),
            };
        Ok(())
    }
}
