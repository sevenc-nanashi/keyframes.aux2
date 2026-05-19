#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}
impl Keyframes {
    pub fn new(num_sections: usize) -> Self {
        let mut keyframes = vec![Keyframe::default(); num_sections];
        keyframes[0].easing = Some("直線移動".to_string());
        Self { keyframes }
    }
    pub fn resize(&mut self, num_sections: usize) {
        if num_sections <= 1 {
            panic!("num_sections must be greater than 1");
        }
        if self.keyframes.len() < num_sections {
            self.keyframes.extend(vec![
                Keyframe::default();
                num_sections - self.keyframes.len()
            ]);
        } else {
            self.keyframes.truncate(num_sections);
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    /// このキーフレームから適用するイージング
    pub easing: Option<String>,
    pub acceleration: bool,
    pub deceleration: bool,
    pub params: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Easing {
    pub name: String,
    pub script: String,
    pub label: Option<String>,
    pub has_speed: bool,
    pub default_acceleration: bool,
    pub default_deceleration: bool,
    pub has_timecontrol: bool,
    pub ignore_midpoints: bool,
    pub params: indexmap::IndexMap<String, f64>,
}

impl Easing {
    pub fn from_script(name: &str, script: &str) -> Easing {
        let mut easing = Easing {
            name: name.to_string(),
            script: script.to_string(),
            label: None,
            has_speed: false,
            default_acceleration: false,
            default_deceleration: false,
            has_timecontrol: false,
            ignore_midpoints: false,
            params: indexmap::IndexMap::new(),
        };

        for line in script.lines() {
            if let Some((_, default_acceleration, default_deceleration)) =
                lazy_regex::regex_captures!(r"--speed:([01]),([01])", line.trim())
            {
                easing.has_speed = true;
                easing.default_acceleration = default_acceleration == "1";
                easing.default_deceleration = default_deceleration == "1";
            }
            if line.trim() == "--timecontrol" {
                easing.has_timecontrol = true;
            }
            if let Some((_, param_name, param_value)) =
                lazy_regex::regex_captures!(r"--param:(\w+),(\d*\.?\d+)", line.trim())
            {
                let param_value: f64 = param_value.parse().unwrap_or(0.0);
                easing.params.insert(param_name.to_string(), param_value);
            }
            if let Some((_, param_captures)) =
                lazy_regex::regex_captures!(r"--param:(\d*\.?\d+)", line.trim())
            {
                let param_value: f64 = param_captures.parse().unwrap_or(0.0);
                easing.params.insert("設定値".to_string(), param_value);
            }
            if line.trim() == "--twopoint" {
                easing.ignore_midpoints = true;
            }
            if let Some(label) = line.strip_prefix("--label:") {
                easing.label = Some(label.trim().to_string());
            }
        }

        easing
    }
    pub fn from_multi_script(multi_script: &str) -> Vec<Easing> {
        let mut current_script: Option<(String, String)> = None;
        let mut easings = Vec::new();
        for line in multi_script.lines() {
            if let Some(script_name) = line.strip_prefix("@") {
                if let Some((name, script)) = current_script.take() {
                    easings.push(Self::from_script(&name, &script));
                }
                let script_name = script_name.trim();
                if !script_name.is_empty() {
                    current_script = Some((script_name.to_string(), String::new()));
                }
            } else if let Some((_, script)) = &mut current_script {
                script.push_str(line);
                script.push('\n');
            }
        }

        if let Some((name, script)) = current_script.take() {
            easings.push(Self::from_script(&name, &script));
        }

        easings
    }
}
