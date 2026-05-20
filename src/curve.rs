#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}
impl Keyframes {
    pub fn new(num_sections: usize) -> Self {
        let mut keyframes = vec![Keyframe::default(); num_sections];
        keyframes[0] = Keyframe::Easing(EasingKeyframeInfo::default());
        Self { keyframes }
    }
    pub fn resize(&mut self, num_keyframes: usize) {
        if num_keyframes <= 1 {
            panic!("num_keyframes must be greater than 1");
        }
        if self.keyframes.len() < num_keyframes {
            self.keyframes.extend(vec![
                Keyframe::default();
                num_keyframes - self.keyframes.len()
            ]);
        } else {
            self.keyframes.truncate(num_keyframes);
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub enum Keyframe {
    Easing(EasingKeyframeInfo),
    Ignored,
    #[default]
    Midpoint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EasingKeyframeInfo {
    pub easing: String,
    pub acceleration: bool,
    pub deceleration: bool,
    pub params: Vec<f64>,
    pub timecontrol: TimeControlBezier,
}
impl Default for EasingKeyframeInfo {
    fn default() -> Self {
        Self {
            easing: "直線移動".to_string(),
            acceleration: false,
            deceleration: false,
            params: Vec::new(),
            timecontrol: TimeControlBezier::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlBezier {
    pub points: Vec<TimeControlBezierPoint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlBezierPoint {
    pub position: [f64; 2],
    pub in_handle: Option<[f64; 2]>,
    pub out_handle: Option<[f64; 2]>,
    pub handles_separated: bool,
}
impl Default for TimeControlBezier {
    fn default() -> Self {
        Self {
            points: vec![
                TimeControlBezierPoint {
                    position: [0.0, 0.0],
                    in_handle: None,
                    out_handle: Some([1.0 / 3.0, 1.0 / 3.0]),
                    handles_separated: false,
                },
                TimeControlBezierPoint {
                    position: [1.0, 1.0],
                    in_handle: Some([2.0 / 3.0, 2.0 / 3.0]),
                    out_handle: None,
                    handles_separated: false,
                },
            ],
        }
    }
}

impl TimeControlBezier {
    pub fn y_at_x(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let segment_index = self.segment_index_at_x(x);
        let mut min_t = 0.0;
        let mut max_t = 1.0;

        for _ in 0..32 {
            let t = (min_t + max_t) / 2.0;
            if self.segment_point_at(segment_index, t)[0] < x {
                min_t = t;
            } else {
                max_t = t;
            }
        }

        self.segment_point_at(segment_index, (min_t + max_t) / 2.0)[1]
    }

    pub fn point_at(&self, t: f64) -> [f64; 2] {
        self.y_point_at_x(t)
    }

    pub fn y_point_at_x(&self, x: f64) -> [f64; 2] {
        let x = x.clamp(0.0, 1.0);
        [x, self.y_at_x(x)]
    }

    pub fn segment_point_at(&self, segment_index: usize, t: f64) -> [f64; 2] {
        let t = t.clamp(0.0, 1.0);
        let mt = 1.0 - t;
        let segment_index = segment_index.min(self.points.len().saturating_sub(2));
        let start = &self.points[segment_index];
        let end = &self.points[segment_index + 1];
        let p0 = start.position;
        let p1 = start.out_handle.unwrap_or(start.position);
        let p2 = end.in_handle.unwrap_or(end.position);
        let p3 = end.position;

        [
            mt.powi(3) * p0[0]
                + 3.0 * mt.powi(2) * t * p1[0]
                + 3.0 * mt * t.powi(2) * p2[0]
                + t.powi(3) * p3[0],
            mt.powi(3) * p0[1]
                + 3.0 * mt.powi(2) * t * p1[1]
                + 3.0 * mt * t.powi(2) * p2[1]
                + t.powi(3) * p3[1],
        ]
    }

    pub fn insert_midpoint(&mut self, after_index: usize) -> usize {
        let after_index = after_index.min(self.points.len().saturating_sub(2));
        let x =
            (self.points[after_index].position[0] + self.points[after_index + 1].position[0]) / 2.0;
        let y = self.y_at_x(x);
        let prev = self.points[after_index].position;
        let next = self.points[after_index + 1].position;
        let handle_delta = [(next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0];
        let new_point = TimeControlBezierPoint {
            position: [x, y],
            in_handle: Some([
                (x - handle_delta[0]).clamp(0.0, 1.0),
                (y - handle_delta[1]).clamp(0.0, 1.0),
            ]),
            out_handle: Some([
                (x + handle_delta[0]).clamp(0.0, 1.0),
                (y + handle_delta[1]).clamp(0.0, 1.0),
            ]),
            handles_separated: false,
        };
        let new_index = after_index + 1;
        self.points.insert(new_index, new_point);
        new_index
    }

    pub fn remove_midpoint(&mut self, index: usize) {
        if index == 0 || index + 1 >= self.points.len() {
            return;
        }
        self.points.remove(index);
    }

    fn segment_index_at_x(&self, x: f64) -> usize {
        if self.points.len() <= 1 {
            return 0;
        }
        self.points
            .windows(2)
            .position(|points| x <= points[1].position[0])
            .unwrap_or(self.points.len() - 2)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timecontrol_bezier_maps_x_to_same_y() {
        let timecontrol = TimeControlBezier::default();

        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((timecontrol.y_at_x(x) - x).abs() < 0.000001);
        }
    }
}
