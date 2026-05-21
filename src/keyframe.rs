#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframes {
    pub keyframes: Vec<Keyframe>,
}
impl Keyframes {
    pub fn new(num_keyframes: usize) -> Self {
        let mut keyframes = vec![Keyframe::default(); num_keyframes];
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
    pub timecontrol: TimeControl,
}
impl Default for EasingKeyframeInfo {
    fn default() -> Self {
        Self {
            easing: "直線移動".to_string(),
            acceleration: false,
            deceleration: false,
            params: Vec::new(),
            timecontrol: TimeControl::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TimeControlMode {
    Bezier,
    Elastic,
    Bounce,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TimeControl {
    Bezier(TimeControlBezier),
    Elastic(TimeControlElastic),
    Bounce(TimeControlBounce),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlElastic {
    pub vertex: [f64; 2],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlBounce {
    pub vertex: [f64; 2],
}

impl Default for TimeControl {
    fn default() -> Self {
        Self::Bezier(TimeControlBezier::default())
    }
}

impl Default for TimeControlElastic {
    fn default() -> Self {
        Self {
            vertex: [0.35, 1.25],
        }
    }
}

impl Default for TimeControlBounce {
    fn default() -> Self {
        Self {
            vertex: [0.55, 0.75],
        }
    }
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

impl TimeControl {
    pub fn default_for_mode(mode: TimeControlMode) -> Self {
        match mode {
            TimeControlMode::Bezier => Self::Bezier(TimeControlBezier::default()),
            TimeControlMode::Elastic => Self::Elastic(TimeControlElastic::default()),
            TimeControlMode::Bounce => Self::Bounce(TimeControlBounce::default()),
        }
    }

    pub fn mode(&self) -> TimeControlMode {
        match self {
            Self::Bezier(_) => TimeControlMode::Bezier,
            Self::Elastic(_) => TimeControlMode::Elastic,
            Self::Bounce(_) => TimeControlMode::Bounce,
        }
    }

    pub fn y_at_x(&self, x: f64) -> f64 {
        match self {
            Self::Bezier(bezier) => bezier.y_at_x(x),
            Self::Elastic(elastic) => Self::interpolated_y_at_x(&elastic.points(), x),
            Self::Bounce(bounce) => bounce.y_at_x(x),
        }
    }

    pub fn point_at(&self, t: f64) -> [f64; 2] {
        self.y_point_at_x(t)
    }

    pub fn y_point_at_x(&self, x: f64) -> [f64; 2] {
        let x = x.clamp(0.0, 1.0);
        [x, self.y_at_x(x)]
    }

    pub fn sampled_points(&self, steps: usize) -> Vec<[f64; 2]> {
        let steps = steps.max(1);
        (0..=steps)
            .map(|i| {
                let x = i as f64 / steps as f64;
                [x, self.y_at_x(x)]
            })
            .collect()
    }

    pub fn editable_vertex(&self) -> Option<[f64; 2]> {
        match self {
            Self::Bezier(_) => None,
            Self::Elastic(elastic) => Some(elastic.vertex),
            Self::Bounce(bounce) => Some(bounce.vertex),
        }
    }

    pub fn set_editable_vertex(&mut self, vertex: [f64; 2]) {
        match self {
            Self::Bezier(_) => {}
            Self::Elastic(elastic) => {
                elastic.vertex = [vertex[0].clamp(0.001, 0.999), vertex[1]];
            }
            Self::Bounce(bounce) => {
                bounce.vertex = [vertex[0].clamp(0.001, 0.999), vertex[1].clamp(0.0, 1.0)];
            }
        }
    }

    fn interpolated_y_at_x(points: &[[f64; 2]], x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let segment_index = points
            .windows(2)
            .position(|points| x <= points[1][0])
            .unwrap_or(points.len().saturating_sub(2));
        let p0 = points[segment_index.saturating_sub(1)];
        let p1 = points[segment_index];
        let p2 = points[(segment_index + 1).min(points.len() - 1)];
        let p3 = points[(segment_index + 2).min(points.len() - 1)];
        if (p2[0] - p1[0]).abs() < f64::EPSILON {
            return p2[1];
        }
        let t = ((x - p1[0]) / (p2[0] - p1[0])).clamp(0.0, 1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * ((2.0 * p1[1])
            + (-p0[1] + p2[1]) * t
            + (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * t2
            + (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * t3)
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

    pub fn sampled_points(&self, steps: usize) -> Vec<[f64; 2]> {
        let steps = steps.max(1);
        (0..=steps)
            .map(|i| {
                let x = i as f64 / steps as f64;
                [x, self.y_at_x(x)]
            })
            .collect()
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

impl TimeControlElastic {
    fn points(&self) -> Vec<[f64; 2]> {
        let vertex = [self.vertex[0].clamp(0.001, 0.999), self.vertex[1]];
        let mut points = vec![[0.0, 0.0], vertex];
        let mut x = vertex[0];
        let mut amplitude = vertex[1] - 1.0;
        let mut sign = -amplitude.signum();
        let mut remaining = 1.0 - vertex[0];
        for _ in 0..4 {
            if remaining <= 0.001 {
                break;
            }
            remaining *= 0.55;
            x = (x + remaining).min(0.999);
            amplitude *= 0.45;
            points.push([x, 1.0 + amplitude.abs() * sign]);
            sign *= -1.0;
        }
        points.push([1.0, 1.0]);
        dedup_monotonic_points(points)
    }
}

impl TimeControlBounce {
    fn y_at_x(&self, x: f64) -> f64 {
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/curve_editor/curve_bounce.cpp
        // より借用。
        //
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/LICENSE.txt
        // ```
        // MIT License
        //
        // Copyright (c) 2022 mimaraka
        //
        // Permission is hereby granted, free of charge, to any person obtaining a copy
        // of this software and associated documentation files (the "Software"), to deal
        // in the Software without restriction, including without limitation the rights
        // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
        // copies of the Software, and to permit persons to whom the Software is
        // furnished to do so, subject to the following conditions:
        //
        // The above copyright notice and this permission notice shall be included in all
        // copies or substantial portions of the Software.
        //
        // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
        // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
        // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
        // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
        // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
        // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
        // SOFTWARE.
        // ```
        let progress = x.clamp(0.0, 1.0);
        let handle_x = self.vertex[0].clamp(0.001, 0.999);
        let handle_y = self.vertex[1].clamp(0.001, 1.0);
        let cor = (1.0 - handle_y).sqrt().clamp(0.001, 0.999);
        let period = 2.0 * handle_x / (cor + 1.0);
        let limit_value = period * (1.0 / (1.0 - cor) - 0.5);

        let active = if limit_value > 1.0 {
            let n = ((1.0 + (cor - 1.0) * (1.0 / period + 0.5)).ln() / cor.ln()).floor();
            progress < period * ((cor.powf(n) - 1.0) / (cor - 1.0) - 0.5)
        } else {
            progress < limit_value
        };
        if !active {
            return 1.0;
        }

        let progress = progress / period;
        let bounce_index = |progress: f64| (((cor - 1.0) * progress + 1.0).ln() / cor.ln()).floor();
        let bounce_offset = |progress: f64| {
            progress + 0.5 + 1.0 / (cor - 1.0)
                - (cor + 1.0) * cor.powf(bounce_index(progress + 0.5)) / (2.0 * cor - 2.0)
        };
        let offset = bounce_offset(progress);
        let ret = 4.0 * offset * offset - cor.powf(2.0 * bounce_index(progress + 0.5));

        (1.0 + ret).clamp(0.0, 1.0)
    }
}

fn dedup_monotonic_points(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut result = Vec::with_capacity(points.len());
    for point in points {
        if result
            .last()
            .is_some_and(|last: &[f64; 2]| (last[0] - point[0]).abs() < 0.000_001)
        {
            continue;
        }
        result.push(point);
    }
    result
}

#[derive(Debug, Clone)]
pub struct Easing {
    pub name: String,
    pub script: String,
    pub label: Option<String>,
    pub path: Option<std::path::PathBuf>,
    pub has_speed: bool,
    pub default_acceleration: bool,
    pub default_deceleration: bool,
    pub has_timecontrol: bool,
    pub ignore_midpoints: bool,
    pub params: indexmap::IndexMap<String, f64>,
}

impl Easing {
    pub fn from_script(path: Option<std::path::PathBuf>, name: &str, script: &str) -> Easing {
        let mut easing = Easing {
            name: name.to_string(),
            script: script.to_string(),
            label: None,
            path,
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
                lazy_regex::regex_captures!(r"--param:([^,]+),(\d*\.?\d+)", line.trim())
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
    pub fn from_multi_script(
        mut path: Option<std::path::PathBuf>,
        multi_script: &str,
    ) -> Vec<Easing> {
        let mut current_script: Option<(String, String)> = None;
        let mut easings = Vec::new();
        for line in multi_script.lines() {
            if let Some(script_name) = line.strip_prefix("@") {
                if let Some((name, script)) = current_script.take() {
                    easings.push(Self::from_script(path.clone(), &name, &script));
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
            easings.push(Self::from_script(path.take(), &name, &script));
        }

        easings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timecontrol_bezier_maps_x_to_same_y() {
        let timecontrol = TimeControl::default();

        for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((timecontrol.y_at_x(x) - x).abs() < 0.000001);
        }
    }

    #[test]
    fn elastic_timecontrol_starts_ends_and_passes_vertex() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);
        let TimeControl::Elastic(elastic) = &timecontrol else {
            unreachable!();
        };
        let vertex = elastic.vertex;

        assert!((timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(vertex[0]) - vertex[1]).abs() < 0.000001);
    }

    #[test]
    fn bounce_timecontrol_starts_ends_and_passes_vertex() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Bounce);
        let TimeControl::Bounce(bounce) = &timecontrol else {
            unreachable!();
        };
        let vertex = bounce.vertex;

        assert!((timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(vertex[0]) - vertex[1]).abs() < 0.000001);
    }

    #[test]
    fn bounce_vertex_y_is_clamped_to_unit_range() {
        let mut timecontrol = TimeControl::default_for_mode(TimeControlMode::Bounce);

        timecontrol.set_editable_vertex([0.5, 1.5]);
        assert_eq!(timecontrol.editable_vertex().unwrap(), [0.5, 1.0]);

        timecontrol.set_editable_vertex([0.5, -0.5]);
        assert_eq!(timecontrol.editable_vertex().unwrap(), [0.5, 0.0]);
    }
}
