use aviutl2_eframe::egui;

#[derive(Clone, Copy)]
pub struct TimeControlViewport {
    pub rect: egui::Rect,
    pub min_y: f64,
    pub max_y: f64,
}

impl TimeControlViewport {
    pub fn graph_to_screen(self, point: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.rect.left() + point[0] as f32 * self.rect.width(),
            self.rect.bottom()
                - ((point[1] - self.min_y) / (self.max_y - self.min_y)) as f32 * self.rect.height(),
        )
    }

    pub fn screen_to_graph(self, point: egui::Pos2) -> [f64; 2] {
        [
            ((point.x - self.rect.left()) / self.rect.width()) as f64,
            self.min_y
                + ((self.rect.bottom() - point.y) / self.rect.height()) as f64
                    * (self.max_y - self.min_y),
        ]
    }
}

#[derive(Clone, Copy)]
pub struct TimeControlDragModifiers {
    pub shift: bool,
    pub alt: bool,
}
