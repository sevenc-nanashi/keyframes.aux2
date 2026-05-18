use anyhow::Context;

mod curve;
mod gui;
mod module;

#[aviutl2::plugin(GenericPlugin)]
struct KeyframesAux2 {
    mod2: aviutl2::generic::SubPlugin<crate::module::KeyframesMod2>,

    current_bank: usize,
    keyframes: std::collections::HashMap<(usize, usize), crate::curve::Keyframes>,
}

static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle = aviutl2::generic::GlobalEditHandle::new();

impl aviutl2::generic::GenericPlugin for KeyframesAux2 {
    fn new(info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();
        Ok(Self {
            mod2: aviutl2::generic::SubPlugin::new_script_module(&info)?,

            current_bank: 0,
            keyframes: std::collections::HashMap::new(),
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "keyframes.aux2".into(),
            information: "".into(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.register_script_module(Some("keyframes.aux2"), &self.mod2);
        EDIT_HANDLE.init(registry.create_edit_handle());
    }

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        let last_bank_id: usize = project.deserialize("last_bank_id").unwrap_or(0);
        self.current_bank = last_bank_id + 1;
        let keyframes: std::collections::HashMap<(usize, usize), crate::curve::Keyframes> =
            project.deserialize("keyframes").unwrap_or_default();
        self.keyframes = keyframes;
    }

    fn on_change_scene(&mut self, edit: &aviutl2::generic::EditSection) {
        self.current_bank += 1;
        let mut used_bank_ids = std::collections::HashSet::new();
        let mut used_keyframes = std::collections::HashSet::new();
        for layer in edit.layers() {
            for (position, object) in layer.objects() {
                if let Err(e) =
                    collect_used_keyframes(edit, object, &mut used_bank_ids, &mut used_keyframes)
                {
                    tracing::error!(
                        "Failed to collect used keyframes for object at position {:?} in layer {:?}: {:?}",
                        position,
                        layer.index,
                        e
                    );
                }
            }
        }
        tracing::info!("Used bank IDs: {:?}", used_bank_ids);
        tracing::info!("Used keyframes: {:?}", used_keyframes);
    }
}

fn collect_used_keyframes(
    edit: &aviutl2::generic::EditSection,
    object: aviutl2::generic::ObjectHandle,
    used_bank_ids: &mut std::collections::HashSet<usize>,
    used_keyframes: &mut std::collections::HashSet<(usize, usize)>,
) -> anyhow::Result<()> {
    let alias = edit
        .get_object_alias_parsed(object)
        .context("Failed to get object alias")?;
    let objects = alias
        .get_table("Object")
        .context("Failed to get Object table")?;
    for object in objects.iter_subtables_as_array() {
        for (_key, value) in object.values() {
            static PATTERN: lazy_regex::Lazy<lazy_regex::regex::Regex> = lazy_regex::lazy_regex!(
                r"keyframes\.aux2,\d+\|(?<bank_id>\d+),(?<keyframes_id>\d+)(?:$|\|)"
            );
            if let Some(captures) = PATTERN.captures(value) {
                let bank_id: usize = captures
                    .name("bank_id")
                    .context("Failed to capture bank_id")?
                    .as_str()
                    .parse()
                    .context("Failed to parse bank_id")?;
                let keyframes_id: usize = captures
                    .name("keyframes_id")
                    .context("Failed to capture keyframes_id")?
                    .as_str()
                    .parse()
                    .context("Failed to parse keyframes_id")?;
                used_bank_ids.insert(bank_id);
                used_keyframes.insert((bank_id, keyframes_id));
            }
        }
    }
    Ok(())
}

aviutl2::register_generic_plugin!(KeyframesAux2);
