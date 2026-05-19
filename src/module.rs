use anyhow::Context;
use aviutl2::module::ScriptModuleFunctions;

#[aviutl2::plugin(ScriptModule)]
pub struct KeyframesMod2 {}

impl aviutl2::module::ScriptModule for KeyframesMod2 {
    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self {})
    }

    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "keyframes.aux2: internal module".into(),
            functions: Self::functions(),
        }
    }
}

#[aviutl2::module::functions]
impl KeyframesMod2 {
    #[expect(clippy::type_complexity)]
    fn get_keyframe(
        &self,
        bank_id: i32,
        track_id: i32,
        index: usize,
    ) -> aviutl2::common::AnyResult<(usize, usize, String, String, bool, bool, Vec<f64>)> {
        let param = crate::KeyframeTrackParams {
            bank_id: bank_id as _,
            keyframes_id: track_id as _,
        };
        let keyframes = crate::KEYFRAMES
            .get(&param)
            .context("keyframes not found")?;
        let (index, keyframe) = keyframes
            .keyframes
            .iter()
            .enumerate()
            .take(index + 1)
            .rfind(|(_, k)| k.easing.is_some())
            .context("failed to find keyframe with easing")?;
        let ends_at = index
            + keyframes.keyframes[(index + 1)..]
                .iter()
                .take_while(|k| k.easing.is_none())
                .count();
        let easing = crate::EASINGS
            .get()
            .context("easings not initialized")?
            .get(keyframe.easing.as_ref().unwrap())
            .context("easing not found")?;
        Ok((
            index,
            ends_at,
            easing.name.clone(),
            easing.script.clone(),
            keyframe.acceleration,
            keyframe.deceleration,
            keyframe.params.clone(),
        ))
    }
}
