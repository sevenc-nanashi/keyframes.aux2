#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    /// このキーフレームから適用するイージング
    pub easing: Option<String>,
}
