#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}
impl Keyframes {
    pub fn new(num_sections: usize) -> Self {
        let mut keyframes = vec![Keyframe { easing: None }; num_sections];
        keyframes[0].easing = Some("直線移動".to_string());
        Self {
            keyframes,
        }
    }
    pub fn resize(&mut self, num_sections: usize) {
        if self.keyframes.len() < num_sections {
            self.keyframes
                .extend(vec![Keyframe { easing: None }; num_sections - self.keyframes.len()]);
        } else {
            self.keyframes.truncate(num_sections);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    /// このキーフレームから適用するイージング
    pub easing: Option<String>,
}
