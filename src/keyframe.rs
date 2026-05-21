// このファイルの内容はmimaraka/aviutl-plugin-curve_editorのコードを参考にしています。
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
pub struct TimeControl {
    pub curve: TimeControlCurve,
    #[serde(default)]
    pub modifier: TimeControlModifier,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TimeControlCurve {
    Bezier(TimeControlBezier),
    Elastic(TimeControlElastic),
    Bounce(TimeControlBounce),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum TimeControlModifier {
    #[default]
    Normal,
    Reverse,
    NormalReverse,
    ReverseNormal,
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
    #[serde(default = "TimeControlElastic::default_amplitude")]
    pub amplitude: f64,
    #[serde(default = "TimeControlElastic::default_frequency")]
    pub frequency: f64,
    #[serde(default = "TimeControlElastic::default_decay")]
    pub decay: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControlBounce {
    pub vertex: [f64; 2],
}

impl Default for TimeControl {
    fn default() -> Self {
        Self {
            curve: TimeControlCurve::Bezier(TimeControlBezier::default()),
            modifier: TimeControlModifier::default(),
        }
    }
}

impl TimeControlModifier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "通常",
            Self::Reverse => "反転",
            Self::NormalReverse => "通常→反転",
            Self::ReverseNormal => "反転→通常",
        }
    }
}

impl Default for TimeControlElastic {
    fn default() -> Self {
        Self {
            amplitude: Self::default_amplitude(),
            frequency: Self::default_frequency(),
            decay: Self::default_decay(),
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
        Self {
            curve: match mode {
                TimeControlMode::Bezier => TimeControlCurve::Bezier(TimeControlBezier::default()),
                TimeControlMode::Elastic => {
                    TimeControlCurve::Elastic(TimeControlElastic::default())
                }
                TimeControlMode::Bounce => TimeControlCurve::Bounce(TimeControlBounce::default()),
            },
            modifier: TimeControlModifier::default(),
        }
    }

    pub fn mode(&self) -> TimeControlMode {
        match self.curve {
            TimeControlCurve::Bezier(_) => TimeControlMode::Bezier,
            TimeControlCurve::Elastic(_) => TimeControlMode::Elastic,
            TimeControlCurve::Bounce(_) => TimeControlMode::Bounce,
        }
    }

    pub fn y_at_x(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self.modifier {
            TimeControlModifier::Normal => self.curve_y_at_x(x),
            TimeControlModifier::Reverse => 1.0 - self.curve_y_at_x(1.0 - x),
            TimeControlModifier::NormalReverse => {
                if x < 0.5 {
                    self.curve_y_at_x(x * 2.0) / 2.0
                } else {
                    1.0 - self.curve_y_at_x((1.0 - x) * 2.0) / 2.0
                }
            }
            TimeControlModifier::ReverseNormal => {
                if x < 0.5 {
                    (1.0 - self.curve_y_at_x(1.0 - x * 2.0)) / 2.0
                } else {
                    0.5 + self.curve_y_at_x(x * 2.0 - 1.0) / 2.0
                }
            }
        }
    }

    pub fn curve_y_at_x(&self, x: f64) -> f64 {
        match &self.curve {
            TimeControlCurve::Bezier(bezier) => bezier.y_at_x(x),
            TimeControlCurve::Elastic(elastic) => elastic.y_at_x(x),
            TimeControlCurve::Bounce(bounce) => bounce.y_at_x(x),
        }
    }

    pub fn curve_sampled_points(&self, steps: usize) -> Vec<[f64; 2]> {
        let steps = steps.max(1);
        if let TimeControlCurve::Bezier(bezier) = &self.curve {
            let mut points = Vec::new();
            for segment_index in 0..bezier.points.len().saturating_sub(1) {
                if points.is_empty() {
                    points.push(bezier.points[segment_index].position);
                }
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    points.push(bezier.segment_point_at(segment_index, t));
                }
            }
            points
        } else {
            (0..=steps)
                .map(|i| {
                    let x = i as f64 / steps as f64;
                    [x, self.curve_y_at_x(x)]
                })
                .collect()
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
        match &self.curve {
            TimeControlCurve::Bezier(_) => None,
            TimeControlCurve::Elastic(_) => None,
            TimeControlCurve::Bounce(bounce) => Some(bounce.vertex),
        }
    }

    pub fn set_editable_vertex(&mut self, vertex: [f64; 2]) {
        match &mut self.curve {
            TimeControlCurve::Bezier(_) => {}
            TimeControlCurve::Elastic(_) => {}
            TimeControlCurve::Bounce(bounce) => {
                bounce.vertex = [vertex[0].clamp(0.001, 0.999), vertex[1].clamp(0.0, 1.0)];
            }
        }
    }
}

impl TimeControlBezier {
    pub fn from_cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            points: vec![
                TimeControlBezierPoint {
                    position: [0.0, 0.0],
                    in_handle: None,
                    out_handle: Some([x1, y1]),
                    handles_separated: false,
                },
                TimeControlBezierPoint {
                    position: [1.0, 1.0],
                    in_handle: Some([x2, y2]),
                    out_handle: None,
                    handles_separated: false,
                },
            ],
        }
    }

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
    fn default_amplitude() -> f64 {
        1.0
    }

    fn default_frequency() -> f64 {
        5.0
    }

    fn default_decay() -> f64 {
        6.0
    }

    pub fn amp_handle(&self) -> [f64; 2] {
        [0.0, 1.0 + self.amplitude.clamp(0.0, 1.0)]
    }

    pub fn freq_decay_handle(&self) -> [f64; 2] {
        let frequency = self.frequency.max(0.5);
        let decay = self.decay.max(1.0);
        [
            (0.5 / frequency).clamp(0.0001, 1.0),
            (1.0 - (-(decay - 1.0) * 0.1).exp()).clamp(0.0, 0.9999),
        ]
    }

    pub fn set_amp_handle_y(&mut self, y: f64) {
        self.amplitude = (y - 1.0).clamp(0.0, 1.0);
    }

    pub fn set_freq_decay_handle(&mut self, point: [f64; 2]) {
        let x = point[0].clamp(0.0001, 1.0);
        let y = point[1].clamp(0.0, 0.9999);
        self.frequency = (0.5 / x).max(0.5);
        self.decay = -10.0 * (1.0 - y).ln() + 1.0;
    }

    fn y_at_x(&self, x: f64) -> f64 {
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/curve_editor/curve_elastic.cpp
        let progress = x.clamp(0.0, 1.0);
        let amplitude = self.amplitude.clamp(0.0, 1.0);
        let frequency = self.frequency.max(0.5);
        let decay = self.decay.max(1.0);
        let omega = 2.0 * std::f64::consts::PI * frequency;
        let exp_k = (-decay).exp();

        let func_elastic = |progress: f64| {
            let coef = if decay == 0.0 {
                1.0 - progress
            } else {
                (exp_k.powf(progress) - exp_k) / (1.0 - exp_k)
            };
            1.0 - coef * (omega * progress).cos()
        };
        let func_elastic_derivative = |progress: f64| {
            let angle = omega * progress;
            let value_cos = angle.cos();
            let value_sin = angle.sin();
            if decay == 0.0 {
                value_cos + omega * (1.0 - progress) * value_sin
            } else {
                let exp_kt = exp_k.powf(progress);
                decay * exp_kt * value_cos + omega * (exp_kt - exp_k) * value_sin
            }
        };
        let func_elastic_derivative_2 = |progress: f64| {
            let angle = omega * progress;
            let value_cos = angle.cos();
            let value_sin = angle.sin();
            if decay == 0.0 {
                omega * omega * (1.0 - progress) * value_cos - 2.0 * omega * value_sin
            } else {
                let exp_kt = exp_k.powf(progress);
                let omega_sq = omega * omega;
                ((omega_sq - decay * decay) * exp_kt - omega_sq * exp_k) * value_cos
                    - 2.0 * omega * decay * exp_kt * value_sin
            }
        };

        let mut extremum_t = (0.5 - (decay / frequency).sqrt() * 0.05) / frequency;
        for _ in 0..3 {
            extremum_t -=
                func_elastic_derivative(extremum_t) / func_elastic_derivative_2(extremum_t);
        }
        let extremum_x = func_elastic(extremum_t);
        let value = func_elastic(progress);
        let ret = if progress < extremum_t {
            (amplitude * (extremum_x - 1.0) + 1.0) / extremum_x * value
        } else {
            amplitude * (value - 1.0) + 1.0
        };

        ret.clamp(0.0, 2.0)
    }
}

impl TimeControlBounce {
    fn y_at_x(&self, x: f64) -> f64 {
        // https://github.com/mimaraka/aviutl-plugin-curve_editor/blob/main/curve_editor/curve_bounce.cpp
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

#[derive(Debug, Clone)]
pub struct TimeControlPreset {
    pub name: &'static str,
    pub timecontrol: TimeControl,
}

pub fn timecontrol_presets() -> Vec<TimeControlPreset> {
    fn bezier(
        name: &'static str,
        modifier: TimeControlModifier,
        cubic_bezier: [f64; 4],
    ) -> TimeControlPreset {
        TimeControlPreset {
            name,
            timecontrol: TimeControl {
                curve: TimeControlCurve::Bezier(TimeControlBezier::from_cubic_bezier(
                    cubic_bezier[0],
                    cubic_bezier[1],
                    cubic_bezier[2],
                    cubic_bezier[3],
                )),
                modifier,
            },
        }
    }
    fn elastic(name: &'static str, modifier: TimeControlModifier) -> TimeControlPreset {
        TimeControlPreset {
            name,
            timecontrol: TimeControl {
                curve: TimeControlCurve::Elastic(TimeControlElastic::default()),
                modifier,
            },
        }
    }
    fn bounce(name: &'static str, modifier: TimeControlModifier) -> TimeControlPreset {
        TimeControlPreset {
            name,
            timecontrol: TimeControl {
                curve: TimeControlCurve::Bounce(TimeControlBounce::default()),
                modifier,
            },
        }
    }

    vec![
        bezier(
            "[1] linear",
            TimeControlModifier::Normal,
            [0.25, 0.25, 0.75, 0.75],
        ),
        bezier(
            "[2] easeInSine",
            TimeControlModifier::Normal,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[3] easeOutSine",
            TimeControlModifier::Reverse,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[4] easeInOutSine",
            TimeControlModifier::NormalReverse,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[5] easeOutInSine",
            TimeControlModifier::ReverseNormal,
            [0.12, 0.0, 0.39, 0.0],
        ),
        bezier(
            "[6] easeInQuad",
            TimeControlModifier::Normal,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[7] easeOutQuad",
            TimeControlModifier::Reverse,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[8] easeInOutQuad",
            TimeControlModifier::NormalReverse,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[9] easeOutInQuad",
            TimeControlModifier::ReverseNormal,
            [0.11, 0.0, 0.5, 0.0],
        ),
        bezier(
            "[10] easeInCubic",
            TimeControlModifier::Normal,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[11] easeOutCubic",
            TimeControlModifier::Reverse,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[12] easeInOutCubic",
            TimeControlModifier::NormalReverse,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[13] easeOutInCubic",
            TimeControlModifier::ReverseNormal,
            [0.32, 0.0, 0.67, 0.0],
        ),
        bezier(
            "[14] easeInQuart",
            TimeControlModifier::Normal,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[15] easeOutQuart",
            TimeControlModifier::Reverse,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[16] easeInOutQuart",
            TimeControlModifier::NormalReverse,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[17] easeOutInQuart",
            TimeControlModifier::ReverseNormal,
            [0.5, 0.0, 0.75, 0.0],
        ),
        bezier(
            "[18] easeInQuint",
            TimeControlModifier::Normal,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[19] easeOutQuint",
            TimeControlModifier::Reverse,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[20] easeInOutQuint",
            TimeControlModifier::NormalReverse,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[21] easeOutInQuint",
            TimeControlModifier::ReverseNormal,
            [0.64, 0.0, 0.78, 0.0],
        ),
        bezier(
            "[22] easeInExpo",
            TimeControlModifier::Normal,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[23] easeOutExpo",
            TimeControlModifier::Reverse,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[24] easeInOutExpo",
            TimeControlModifier::NormalReverse,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[25] easeOutInExpo",
            TimeControlModifier::ReverseNormal,
            [0.7, 0.0, 0.84, 0.0],
        ),
        bezier(
            "[26] easeInCirc",
            TimeControlModifier::Normal,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[27] easeOutCirc",
            TimeControlModifier::Reverse,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[28] easeInOutCirc",
            TimeControlModifier::NormalReverse,
            [0.55, 0.0, 1.0, 0.45],
        ),
        bezier(
            "[29] easeOutInCirc",
            TimeControlModifier::ReverseNormal,
            [0.55, 0.0, 1.0, 0.45],
        ),
        elastic("[30] easeInElastic", TimeControlModifier::Reverse),
        elastic("[31] easeOutElastic", TimeControlModifier::Normal),
        elastic("[32] easeInOutElastic", TimeControlModifier::ReverseNormal),
        elastic("[33] easeOutInElastic", TimeControlModifier::NormalReverse),
        bezier(
            "[34] easeInBack",
            TimeControlModifier::Normal,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[35] easeOutBack",
            TimeControlModifier::Reverse,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[36] easeInOutBack",
            TimeControlModifier::NormalReverse,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bezier(
            "[37] easeOutInBack",
            TimeControlModifier::ReverseNormal,
            [0.36, 0.0, 0.66, -0.56],
        ),
        bounce("[38] easeInBounce", TimeControlModifier::Reverse),
        bounce("[39] easeOutBounce", TimeControlModifier::Normal),
        bounce("[40] easeInOutBounce", TimeControlModifier::ReverseNormal),
        bounce("[41] easeOutInBounce", TimeControlModifier::NormalReverse),
    ]
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
    fn elastic_timecontrol_starts_and_ends_at_anchors() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);

        assert!((timecontrol.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((timecontrol.y_at_x(1.0) - 1.0).abs() < 0.000001);
    }

    #[test]
    fn elastic_handles_update_parameters() {
        let mut timecontrol = TimeControl::default_for_mode(TimeControlMode::Elastic);
        let TimeControlCurve::Elastic(elastic) = &mut timecontrol.curve else {
            unreachable!();
        };

        elastic.set_amp_handle_y(1.5);
        elastic.set_freq_decay_handle([0.25, 0.5]);

        assert!((elastic.amplitude - 0.5).abs() < 0.000001);
        assert!((elastic.frequency - 2.0).abs() < 0.000001);
        assert!((elastic.freq_decay_handle()[1] - 0.5).abs() < 0.000001);
    }

    #[test]
    fn bounce_timecontrol_starts_ends_and_passes_vertex() {
        let timecontrol = TimeControl::default_for_mode(TimeControlMode::Bounce);
        let TimeControlCurve::Bounce(bounce) = &timecontrol.curve else {
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

    #[test]
    fn timecontrol_modifier_transforms_curve() {
        let normal = TimeControl::default_for_mode(TimeControlMode::Bounce);
        let mut reverse = normal.clone();
        reverse.modifier = TimeControlModifier::Reverse;
        let mut normal_reverse = normal.clone();
        normal_reverse.modifier = TimeControlModifier::NormalReverse;
        let mut reverse_normal = normal.clone();
        reverse_normal.modifier = TimeControlModifier::ReverseNormal;

        for x in [0.125, 0.25, 0.75] {
            assert!((reverse.y_at_x(x) - (1.0 - normal.y_at_x(1.0 - x))).abs() < 0.000001);
        }
        assert!((normal_reverse.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((normal_reverse.y_at_x(0.5) - 0.5).abs() < 0.000001);
        assert!((normal_reverse.y_at_x(1.0) - 1.0).abs() < 0.000001);
        assert!((reverse_normal.y_at_x(0.0) - 0.0).abs() < 0.000001);
        assert!((reverse_normal.y_at_x(0.5) - 0.5).abs() < 0.000001);
        assert!((reverse_normal.y_at_x(1.0) - 1.0).abs() < 0.000001);
    }
}
