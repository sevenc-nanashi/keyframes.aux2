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
impl KeyframesMod2 {}
