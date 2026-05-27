use aviutl2_eframe::egui;

#[derive(Clone, Copy)]
pub(crate) struct TimeControlViewport {
    pub(crate) rect: egui::Rect,
    pub(crate) min_y: f64,
    pub(crate) max_y: f64,
}

impl TimeControlViewport {
    pub(crate) fn graph_to_screen(self, point: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.rect.left() + point[0] as f32 * self.rect.width(),
            self.rect.bottom()
                - ((point[1] - self.min_y) / (self.max_y - self.min_y)) as f32 * self.rect.height(),
        )
    }

    pub(crate) fn screen_to_graph(self, point: egui::Pos2) -> [f64; 2] {
        [
            ((point.x - self.rect.left()) / self.rect.width()) as f64,
            self.min_y
                + ((self.rect.bottom() - point.y) / self.rect.height()) as f64
                    * (self.max_y - self.min_y),
        ]
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TimeControlDragModifiers {
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}
