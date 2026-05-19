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
    ) -> aviutl2::common::AnyResult<(Vec<i32>, String, String, bool, bool, Vec<f64>)> {
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
            .rfind(|(_, k)| matches!(k, crate::curve::Keyframe::Easing(_)))
            .expect("first keyframe must be easing");
        let crate::curve::Keyframe::Easing(keyframe) = keyframe else {
            unreachable!()
        };
        let mut indices = vec![index as i32];
        for i in (index + 1)..keyframes.keyframes.len() {
            match &keyframes.keyframes[i] {
                crate::curve::Keyframe::Easing(_) => {
                    indices.push(i as i32);
                    break;
                }
                crate::curve::Keyframe::Midpoint => indices.push(i as i32),
                crate::curve::Keyframe::Ignored => (),
            }
        }
        aviutl2::ldbg!(&indices);
        let easing = crate::EASINGS
            .get()
            .context("easings not initialized")?
            .get(&keyframe.easing)
            .context("easing not found")?;
        Ok((
            indices,
            easing.name.clone(),
            easing.script.clone(),
            keyframe.acceleration,
            keyframe.deceleration,
            keyframe.params.clone(),
        ))
    }
}
