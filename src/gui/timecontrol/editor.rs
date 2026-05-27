use super::*;

impl KeyframesGui {
    pub(crate) fn show_timecontrol_bezier_editor(
        ui: &mut egui::Ui,
        timecontrol: &mut crate::keyframe::TimeControl,
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
        let (content_min_y, content_max_y) = Self::timecontrol_editor_vertical_bounds(timecontrol);
        let content_y_range = (content_max_y - content_min_y).max(0.000_001);
        *vertical_zoom = (*vertical_zoom).clamp(1.0, 8.0);
        let visible_y_range = content_y_range / *vertical_zoom;
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
                    *vertical_zoom = (*vertical_zoom * zoom_delta).clamp(1.0, 8.0);
                } else if scroll_delta.abs() > f64::EPSILON {
                    *vertical_zoom = (*vertical_zoom * (scroll_delta * 0.01).exp()).clamp(1.0, 8.0);
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
            if Self::show_timecontrol_segment_mode_menu(ui, timecontrol, *selected_point) {
                changed = true;
                commit_requested = true;
            }
            if Self::show_timecontrol_modifier_menu(ui, timecontrol) {
                changed = true;
                commit_requested = true;
            }
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
        Self::draw_timecontrol_curve(&painter, timecontrol, viewport, false);
        Self::draw_timecontrol_modifier_label(&painter, timecontrol, viewport);
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
                movable_y_range,
            );
        changed |= handle_changed;
        commit_requested |= handle_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            if matches!(
                timecontrol.segment_mode(segment_index),
                Some(crate::keyframe::TimeControlMode::Elastic)
            ) {
                let (elastic_changed, elastic_commit_requested) =
                    Self::show_timecontrol_elastic_handles(
                        ui,
                        &painter,
                        timecontrol,
                        segment_index,
                        selected_point,
                        viewport,
                        vertical_scroll,
                        movable_y_range,
                    );
                changed |= elastic_changed;
                commit_requested |= elastic_commit_requested;
            } else if matches!(
                timecontrol.segment_mode(segment_index),
                Some(crate::keyframe::TimeControlMode::Bounce)
            ) {
                let (vertex_changed, vertex_commit_requested) = Self::show_timecontrol_vertex(
                    ui,
                    &painter,
                    timecontrol,
                    segment_index,
                    selected_point,
                    viewport,
                    vertical_scroll,
                    movable_y_range,
                );
                changed |= vertex_changed;
                commit_requested |= vertex_commit_requested;
            }
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
                movable_y_range,
            );
        changed |= anchor_changed;
        commit_requested |= anchor_commit_requested;
        if structure_changed {
            return (changed, commit_requested);
        }

        (changed, commit_requested)
    }

    pub(crate) fn timecontrol_editor_vertical_bounds(
        timecontrol: &crate::keyframe::TimeControl,
    ) -> (f64, f64) {
        let mut min_y = 0.0_f64;
        let mut max_y = 1.0_f64;
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            let start = timecontrol.points[segment_index].position;
            let end = timecontrol.points[segment_index + 1].position;
            let (segment_min_y, segment_max_y) = match timecontrol.points[segment_index].outgoing {
                Some(crate::keyframe::TimeControlSegment::Elastic(_)) => {
                    let elastic_max_y = start[1] + (end[1] - start[1]) * 2.0;
                    (start[1].min(elastic_max_y), start[1].max(elastic_max_y))
                }
                Some(crate::keyframe::TimeControlSegment::Bounce(_)) => {
                    (start[1].min(end[1]), start[1].max(end[1]))
                }
                _ => {
                    let mut segment_min_y = start[1].min(end[1]);
                    let mut segment_max_y = start[1].max(end[1]);
                    for position in [
                        timecontrol.points[segment_index].out_handle,
                        timecontrol.points[segment_index + 1].in_handle,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        segment_min_y = segment_min_y.min(position[1]);
                        segment_max_y = segment_max_y.max(position[1]);
                    }
                    (segment_min_y, segment_max_y)
                }
            };
            min_y = min_y.min(segment_min_y);
            max_y = max_y.max(segment_max_y);
        }
        (min_y, max_y)
    }

    pub(crate) fn timecontrol_vertical_bounds(
        timecontrol: &crate::keyframe::TimeControl,
    ) -> (f64, f64) {
        let mut min_y = 0.0_f64;
        let mut max_y = 1.0_f64;
        for position in timecontrol.sampled_points(96) {
            min_y = min_y.min(position[1]);
            max_y = max_y.max(position[1]);
        }
        (min_y, max_y)
    }
}
