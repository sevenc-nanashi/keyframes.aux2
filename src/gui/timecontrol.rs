use super::*;
use anyhow::Context;
use aviutl2_eframe::egui;

#[derive(Clone, Copy)]
struct TimeControlViewport {
    rect: egui::Rect,
    min_y: f64,
    max_y: f64,
}

impl TimeControlViewport {
    fn graph_to_screen(self, point: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.rect.left() + point[0] as f32 * self.rect.width(),
            self.rect.bottom()
                - ((point[1] - self.min_y) / (self.max_y - self.min_y)) as f32 * self.rect.height(),
        )
    }

    fn screen_to_graph(self, point: egui::Pos2) -> [f64; 2] {
        [
            ((point.x - self.rect.left()) / self.rect.width()) as f64,
            self.min_y
                + ((self.rect.bottom() - point.y) / self.rect.height()) as f64
                    * (self.max_y - self.min_y),
        ]
    }
}

#[derive(Clone, Copy)]
struct TimeControlDragModifiers {
    shift: bool,
    alt: bool,
}

impl KeyframesGui {
    fn update_track_keyframes_by_target(
        target: &TimeControlEditorTarget,
        new_keyframes: crate::keyframe::Keyframes,
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
            Some(crate::keyframe::Keyframe::Easing(kf_info)) => kf_info,
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

        target.vertical_zoom = target.vertical_zoom.clamp(0.25, 8.0);
        target.vertical_scroll = target.vertical_scroll.clamp(0.0, 1.0);
        let content_size = ui.available_size();
        let total_width = content_size.x;
        let content_height = content_size.y;
        let separator_width = 8.0;
        if !target.preset_panel_width.is_finite() {
            target.preset_panel_width = (total_width - content_height - separator_width).max(0.0);
        }
        target.preset_panel_width = target
            .preset_panel_width
            .clamp(0.0, (total_width - separator_width).max(0.0));
        let preset_width = target.preset_panel_width;
        let editor_width = (total_width - preset_width - separator_width).max(0.0);
        let (content_rect, _) = ui.allocate_exact_size(content_size, egui::Sense::hover());
        let editor_rect =
            egui::Rect::from_min_size(content_rect.min, egui::vec2(editor_width, content_height));
        let separator_rect = egui::Rect::from_min_size(
            egui::pos2(editor_rect.right(), content_rect.top()),
            egui::vec2(separator_width, content_height),
        );
        let preset_rect = egui::Rect::from_min_size(
            egui::pos2(separator_rect.right(), content_rect.top()),
            egui::vec2(preset_width, content_height),
        );
        let mut result = (false, false);

        if editor_width > 1.0 {
            let mut editor_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(editor_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            editor_ui.set_clip_rect(editor_rect);
            result = Self::show_timecontrol_bezier_editor(
                &mut editor_ui,
                &mut target.timecontrol,
                &mut target.selected_point,
                &mut target.context_menu_position,
                &mut target.vertical_zoom,
                &mut target.vertical_scroll,
            );
        }

        let separator_response = ui.interact(
            separator_rect,
            ui.id().with("timecontrol_editor_separator"),
            egui::Sense::drag(),
        );
        if separator_response.hovered() || separator_response.dragged() {
            ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
        ui.painter().line_segment(
            [separator_rect.center_top(), separator_rect.center_bottom()],
            egui::Stroke::new(1.0, GUI_COLORS.grid_line),
        );
        if separator_response.dragged() {
            target.preset_panel_width = (target.preset_panel_width
                - separator_response.drag_delta().x)
                .clamp(0.0, (total_width - separator_width).max(0.0));
        }

        if preset_width > 1.0 {
            let mut preset_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(preset_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            preset_ui.set_clip_rect(preset_rect);
            preset_ui.label("プリセット");
        }
        let (changed, commit_requested) = result;
        target.dirty |= changed;

        if commit_requested && target.dirty {
            let Some(mut new_keyframes) = crate::KEYFRAMES
                .get(&target.params)
                .map(|keyframes| keyframes.clone())
            else {
                self.timecontrol_editor = None;
                return;
            };
            let Some(crate::keyframe::Keyframe::Easing(kf_info)) =
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        vertical_zoom: &mut f64,
        vertical_scroll: &mut f64,
    ) -> (bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;
        *selected_point = (*selected_point).min(timecontrol.points.len().saturating_sub(1));

        let available_size = ui.available_size();
        if available_size.x <= f32::EPSILON || available_size.y <= f32::EPSILON {
            return (false, false);
        }
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let mut rect = response.rect.shrink(response.rect.height() * 0.1);
        rect.set_left(rect.left() + rect.height() * 0.1);
        let visible_y_range = 1.0 / (*vertical_zoom).clamp(0.25, 8.0);
        let (content_min_y, content_max_y) = Self::timecontrol_vertical_bounds(timecontrol);
        let content_y_range = content_max_y - content_min_y;
        let (scroll_min_y, scroll_max_y) = if content_y_range >= visible_y_range {
            (content_min_y, content_max_y - visible_y_range)
        } else {
            (content_max_y - visible_y_range, content_min_y)
        };
        let movable_y_range = scroll_max_y - scroll_min_y;
        if response.hovered() {
            let (scroll_delta, zoom_delta, ctrl) = ui.input(|i| {
                (
                    i.smooth_scroll_delta().y as f64,
                    i.zoom_delta() as f64,
                    i.modifiers.ctrl,
                )
            });
            if ctrl {
                if (zoom_delta - 1.0).abs() > f64::EPSILON {
                    *vertical_zoom = (*vertical_zoom * zoom_delta).clamp(0.25, 8.0);
                } else if scroll_delta.abs() > f64::EPSILON {
                    *vertical_zoom =
                        (*vertical_zoom * (scroll_delta * 0.01).exp()).clamp(0.25, 8.0);
                }
            } else if scroll_delta.abs() > f64::EPSILON
                && rect.height() > f32::EPSILON
                && movable_y_range > f64::EPSILON
            {
                let scroll_ratio =
                    scroll_delta / rect.height() as f64 * visible_y_range / movable_y_range;
                *vertical_scroll = (*vertical_scroll + scroll_ratio).clamp(0.0, 1.0);
            }
        }
        *vertical_scroll = (*vertical_scroll).clamp(0.0, 1.0);
        let min_y = scroll_min_y + movable_y_range * *vertical_scroll;
        let max_y = min_y + visible_y_range;
        let viewport = TimeControlViewport { rect, min_y, max_y };

        if response.secondary_clicked() {
            *context_menu_position = response
                .interact_pointer_pos()
                .map(|pos| viewport.screen_to_graph(pos));
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

        Self::draw_timecontrol_grid(&painter, response.rect, viewport);
        Self::draw_timecontrol_curve(&painter, timecontrol, viewport);
        Self::draw_timecontrol_control_lines(&painter, timecontrol, viewport);

        let (handle_changed, handle_commit_requested, structure_changed) =
            Self::show_timecontrol_handles(
                ui,
                &painter,
                timecontrol,
                selected_point,
                context_menu_position,
                viewport,
                vertical_scroll,
            );
        changed |= handle_changed;
        commit_requested |= handle_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        let (anchor_changed, anchor_commit_requested, structure_changed) =
            Self::show_timecontrol_anchors(
                ui,
                &painter,
                timecontrol,
                selected_point,
                context_menu_position,
                viewport,
                vertical_scroll,
            );
        changed |= anchor_changed;
        commit_requested |= anchor_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        (changed, commit_requested)
    }

    fn timecontrol_vertical_bounds(timecontrol: &crate::keyframe::TimeControlBezier) -> (f64, f64) {
        let mut min_y = 0.0_f64;
        let mut max_y = 1.0_f64;
        for point in &timecontrol.points {
            for position in [Some(point.position), point.in_handle, point.out_handle]
                .into_iter()
                .flatten()
            {
                min_y = min_y.min(position[1]);
                max_y = max_y.max(position[1]);
            }
        }
        (min_y, max_y)
    }

    fn draw_timecontrol_grid(
        painter: &egui::Painter,
        response_rect: egui::Rect,
        viewport: TimeControlViewport,
    ) {
        let grid_stroke = egui::Stroke::new(1.0, GUI_COLORS.grid_line);
        let strong_grid_stroke = egui::Stroke::new(1.5, GUI_COLORS.grid_line.linear_multiply(1.35));
        let graph_rect = egui::Rect::from_min_max(
            viewport.graph_to_screen([0.0, 1.0]),
            viewport.graph_to_screen([1.0, 0.0]),
        );

        for i in 1..4 {
            let x = viewport.rect.left() + viewport.rect.width() * i as f32 / 4.0;
            painter.line_segment(
                [
                    egui::pos2(x, response_rect.top()),
                    egui::pos2(x, response_rect.bottom()),
                ],
                grid_stroke,
            );
        }

        let start_grid_y = (viewport.min_y * 4.0).floor() as i32;
        let end_grid_y = (viewport.max_y * 4.0).ceil() as i32;
        let grid_label_font = egui::FontId::proportional(11.0);
        for i in start_grid_y..=end_grid_y {
            let y = i as f64 / 4.0;
            let stroke = if y % 1.0 == 0.0 {
                strong_grid_stroke
            } else {
                grid_stroke
            };
            let y_pos = viewport.graph_to_screen([0.0, y]).y;
            painter.line_segment(
                [
                    egui::pos2(viewport.rect.left(), y_pos),
                    egui::pos2(viewport.rect.right(), y_pos),
                ],
                stroke,
            );
            painter.text(
                egui::pos2(viewport.rect.left() - 6.0, y_pos),
                egui::Align2::RIGHT_CENTER,
                Self::format_timecontrol_grid_label(y),
                grid_label_font.clone(),
                GUI_COLORS.text,
            );
        }

        for x in [viewport.rect.left(), viewport.rect.right()] {
            painter.line_segment(
                [
                    egui::pos2(x, response_rect.top()),
                    egui::pos2(x, response_rect.bottom()),
                ],
                strong_grid_stroke,
            );
        }
        painter.rect_stroke(
            graph_rect,
            0.0,
            strong_grid_stroke,
            egui::StrokeKind::Inside,
        );
    }

    fn draw_timecontrol_curve(
        painter: &egui::Painter,
        timecontrol: &crate::keyframe::TimeControlBezier,
        viewport: TimeControlViewport,
    ) {
        let curve_stroke = egui::Stroke::new(2.0, GUI_COLORS.zoom_gauge);
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let mut previous = viewport.graph_to_screen(timecontrol.points[segment_index].position);
            for i in 1..=24 {
                let t = i as f64 / 24.0;
                let point = timecontrol.segment_point_at(segment_index, t);
                let current = viewport.graph_to_screen(point);
                painter.line_segment([previous, current], curve_stroke);
                previous = current;
            }
        }
    }

    fn draw_timecontrol_control_lines(
        painter: &egui::Painter,
        timecontrol: &crate::keyframe::TimeControlBezier,
        viewport: TimeControlViewport,
    ) {
        let control_stroke = egui::Stroke::new(1.0, GUI_COLORS.anchor_line);
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let start = viewport.graph_to_screen(timecontrol.points[segment_index].position);
            let end = viewport.graph_to_screen(timecontrol.points[segment_index + 1].position);
            let out_handle = viewport.graph_to_screen(
                timecontrol.points[segment_index]
                    .out_handle
                    .unwrap_or(timecontrol.points[segment_index].position),
            );
            let in_handle = viewport.graph_to_screen(
                timecontrol.points[segment_index + 1]
                    .in_handle
                    .unwrap_or(timecontrol.points[segment_index + 1].position),
            );
            painter.line_segment([start, out_handle], control_stroke);
            painter.line_segment([in_handle, end], control_stroke);
        }
    }

    fn show_timecontrol_handles(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControlBezier,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        viewport: TimeControlViewport,
        vertical_scroll: &mut f64,
    ) -> (bool, bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;
        let mut structure_changed = false;

        for point_index in 0..timecontrol.points.len() {
            for handle_kind in [TimeControlHandleKind::In, TimeControlHandleKind::Out] {
                let Some(handle) = (match handle_kind {
                    TimeControlHandleKind::In => timecontrol.points[point_index].in_handle,
                    TimeControlHandleKind::Out => timecontrol.points[point_index].out_handle,
                }) else {
                    continue;
                };

                let handle_pos = viewport.graph_to_screen(handle);
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
                    *context_menu_position = handle_response
                        .interact_pointer_pos()
                        .map(|pos| viewport.screen_to_graph(pos));
                }
                if handle_response.dragged()
                    && let Some(pointer_pos) = handle_response.interact_pointer_pos()
                {
                    let position = Self::timecontrol_drag_position(
                        viewport.screen_to_graph(pointer_pos),
                        Self::timecontrol_drag_modifiers(ui),
                        Some(timecontrol.points[point_index].position[1]),
                    );
                    let new_point = Self::constrain_timecontrol_handle_position(
                        timecontrol,
                        point_index,
                        handle_kind,
                        position,
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
                        Self::set_timecontrol_handle(
                            timecontrol,
                            point_index,
                            handle_kind,
                            new_point,
                        );
                        changed = true;
                        Self::keep_timecontrol_y_in_scroll_range(
                            vertical_scroll,
                            viewport,
                            new_point[1],
                        );
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
                            context_menu_position
                                .unwrap_or_else(|| viewport.screen_to_graph(handle_pos)),
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
                    return (changed, commit_requested, true);
                }
            }
        }

        (changed, commit_requested, false)
    }

    fn show_timecontrol_anchors(
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        timecontrol: &mut crate::keyframe::TimeControlBezier,
        selected_point: &mut usize,
        context_menu_position: &mut Option<[f64; 2]>,
        viewport: TimeControlViewport,
        vertical_scroll: &mut f64,
    ) -> (bool, bool, bool) {
        let mut changed = false;
        let mut commit_requested = false;

        for point_index in 0..timecontrol.points.len() {
            let point = viewport.graph_to_screen(timecontrol.points[point_index].position);
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
                *context_menu_position = handle_response
                    .interact_pointer_pos()
                    .map(|pos| viewport.screen_to_graph(pos));
            }
            if handle_response.dragged()
                && let Some(pointer_pos) = handle_response.interact_pointer_pos()
            {
                let new_position = Self::timecontrol_drag_position(
                    viewport.screen_to_graph(pointer_pos),
                    Self::timecontrol_drag_modifiers(ui),
                    None,
                );
                if Self::move_timecontrol_anchor(timecontrol, point_index, new_position) {
                    changed = true;
                    Self::keep_timecontrol_y_in_scroll_range(
                        vertical_scroll,
                        viewport,
                        new_position[1],
                    );
                }
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
            Self::draw_timecontrol_anchor(
                painter,
                point,
                timecontrol.points[point_index].handles_separated,
                color,
            );
            let before_len = timecontrol.points.len();
            handle_response.context_menu(|ui| {
                if Self::show_timecontrol_anchor_menu(
                    ui,
                    timecontrol,
                    selected_point,
                    context_menu_position.unwrap_or_else(|| viewport.screen_to_graph(point)),
                ) {
                    changed = true;
                    commit_requested = true;
                }
            });
            if timecontrol.points.len() != before_len {
                return (changed, commit_requested, true);
            }
        }

        (changed, commit_requested, false)
    }

    fn draw_timecontrol_anchor(
        painter: &egui::Painter,
        center: egui::Pos2,
        separated_handles: bool,
        color: egui::Color32,
    ) {
        if separated_handles {
            let radius = 6.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(center.x, center.y - radius),
                    egui::pos2(center.x + radius, center.y),
                    egui::pos2(center.x, center.y + radius),
                    egui::pos2(center.x - radius, center.y),
                ],
                color,
                egui::Stroke::NONE,
            ));
        } else {
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::Vec2::splat(10.0)),
                2.0,
                color,
            );
        }
    }

    fn format_timecontrol_grid_label(value: f64) -> String {
        if value.abs() < 0.000_001 {
            "0".to_string()
        } else if (value - value.round()).abs() < 0.000_001 {
            format!("{value:.0}")
        } else {
            format!("{value:.2}")
        }
    }

    fn show_timecontrol_anchor_menu(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
        timecontrol: &crate::keyframe::TimeControlBezier,
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

    fn snap_timecontrol_position(position: [f64; 2], step: f64) -> [f64; 2] {
        [
            (position[0] / step).round() * step,
            (position[1] / step).round() * step,
        ]
    }

    fn timecontrol_drag_modifiers(ui: &egui::Ui) -> TimeControlDragModifiers {
        ui.input(|input| TimeControlDragModifiers {
            shift: input.modifiers.shift,
            alt: input.modifiers.alt,
        })
    }

    fn timecontrol_drag_position(
        mut position: [f64; 2],
        modifiers: TimeControlDragModifiers,
        horizontal_snap_y: Option<f64>,
    ) -> [f64; 2] {
        if modifiers.alt {
            position = Self::snap_timecontrol_position(position, 0.125);
        }
        if let Some(y) = horizontal_snap_y
            && modifiers.shift
        {
            position[1] = y;
        }
        position
    }

    fn keep_timecontrol_y_in_scroll_range(
        vertical_scroll: &mut f64,
        viewport: TimeControlViewport,
        y: f64,
    ) {
        if y < viewport.min_y {
            *vertical_scroll = 0.0;
        } else if y > viewport.max_y {
            *vertical_scroll = 1.0;
        }
    }

    fn move_timecontrol_anchor(
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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

    fn set_timecontrol_handle(
        timecontrol: &mut crate::keyframe::TimeControlBezier,
        point_index: usize,
        handle_kind: TimeControlHandleKind,
        point: [f64; 2],
    ) {
        match handle_kind {
            TimeControlHandleKind::In => {
                timecontrol.points[point_index].in_handle = Some(point);
                if !timecontrol.points[point_index].handles_separated {
                    Self::mirror_timecontrol_handle(timecontrol, point_index, false);
                }
            }
            TimeControlHandleKind::Out => {
                timecontrol.points[point_index].out_handle = Some(point);
                if !timecontrol.points[point_index].handles_separated {
                    Self::mirror_timecontrol_handle(timecontrol, point_index, true);
                }
            }
        }
    }

    fn constrain_all_timecontrol_handles(timecontrol: &mut crate::keyframe::TimeControlBezier) {
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
        timecontrol: &crate::keyframe::TimeControlBezier,
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
        timecontrol: &crate::keyframe::TimeControlBezier,
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
        timecontrol: &mut crate::keyframe::TimeControlBezier,
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
                crate::keyframe::Keyframe::Easing(ref easing) => easing.timecontrol.clone(),
                _ => anyhow::bail!("Target keyframe is not easing"),
            };
        Ok(())
    }
}
