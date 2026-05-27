use super::*;

impl KeyframesGui {
    pub fn draw_timecontrol_grid(
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

    pub fn draw_timecontrol_curve(
        painter: &egui::Painter,
        timecontrol: &crate::keyframe::TimeControl,
        viewport: TimeControlViewport,
        sample_final_curve: bool,
    ) {
        let curve_stroke = egui::Stroke::new(2.0, GUI_COLORS.zoom_gauge);
        let points = if sample_final_curve {
            timecontrol.sampled_points(96)
        } else {
            timecontrol.curve_sampled_points(96)
        };
        for points in points.windows(2) {
            painter.line_segment(
                [
                    viewport.graph_to_screen(points[0]),
                    viewport.graph_to_screen(points[1]),
                ],
                curve_stroke,
            );
        }
    }

    pub fn draw_timecontrol_control_lines(
        painter: &egui::Painter,
        timecontrol: &crate::keyframe::TimeControl,
        viewport: TimeControlViewport,
    ) {
        let control_stroke = egui::Stroke::new(1.0, GUI_COLORS.anchor_line);
        for segment_index in 0..timecontrol.points.len().saturating_sub(1) {
            if timecontrol.segment_mode(segment_index)
                != Some(crate::keyframe::TimeControlMode::Bezier)
            {
                continue;
            }
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
}
