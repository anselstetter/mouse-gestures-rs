use crate::config::config::{Binding, Point};
use crate::info;
use crate::recognizer::recognizer::{Candidate, Recognizer};
use std::cmp;
use std::f64::consts::PI;

const MOV_AVG_COEFFICIENT: f64 = 0.8;
const MOV_AVG_COEFFICIENT_COMP: f64 = 1. - MOV_AVG_COEFFICIENT;
const DIFF_MIN_WITH_SECOND: f64 = 0.05;
const DIFF_MAX: f64 = 0.8;
const SHAPE_MIN_SIZE: usize = 8;

pub struct AngleRecognizer {}

impl AngleRecognizer {
    pub fn new() -> Self {
        Self {}
    }
}

// Modified from https://github.com/jersou/mouse-actions
// Original work Copyright (c) 2023 Jérémy Soulary
// Modifications Copyright (c) 2025 Florian Anselstetter
// Licensed under the MIT License.
impl AngleRecognizer {
    fn compare_angles(&self, vec_a: &[f64], vec_b: &[f64]) -> f64 {
        let (shorter, longer) = if vec_a.len() < vec_b.len() {
            (vec_a, vec_b)
        } else {
            (vec_b, vec_a)
        };

        let len_short = shorter.len() as f64;
        let len_long = longer.len() as f64;

        let total_diff: f64 = shorter
            .iter()
            .enumerate()
            .map(|(i, &angle_a)| {
                let progress = i as f64 / len_short;
                let idx = (progress * len_long) as usize;

                match longer.get(idx) {
                    Some(&angle_b) => {
                        let diff = (angle_a - angle_b).abs();
                        if diff > PI {
                            2.0 * PI - diff
                        } else {
                            diff
                        }
                    }
                    None => PI, // Fallback in case of out-of-bounds
                }
            })
            .sum();

        total_diff / len_short
    }

    fn compare_angles_with_offset(&self, vec_a: &[f64], vec_b: &[f64]) -> f64 {
        let mut diff_values = Vec::with_capacity(25);

        // Always compare the smaller with offset against the larger
        let (shorter, longer) = if vec_a.len() < vec_b.len() {
            (vec_a, vec_b)
        } else {
            (vec_b, vec_a)
        };

        // Initial comparison without offset
        diff_values.push(self.compare_angles(shorter, longer));

        let offset_max = cmp::min(20, shorter.len() / 10);

        for offset in (2..offset_max).step_by(2) {
            // Offset forward
            diff_values.push(self.compare_angles(&shorter[offset..], longer));
            // Offset backward
            diff_values.push(self.compare_angles(&shorter[..shorter.len() - offset], longer));
        }

        // Return the minimum difference (safe unwrap because we always push at least one value)
        diff_values
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn points_to_angles(&self, points: &[Point]) -> Vec<f64> {
        let mut angles = Vec::with_capacity(points.len());

        if points.len() < 2 {
            return angles;
        }

        let mut rx = 0.0;
        let mut ry = 0.0;

        for window in points.windows(2) {
            let (prev, curr) = (&window[0], &window[1]);

            rx = MOV_AVG_COEFFICIENT * rx + MOV_AVG_COEFFICIENT_COMP * (curr.x - prev.x) as f64;
            ry = MOV_AVG_COEFFICIENT * ry + MOV_AVG_COEFFICIENT_COMP * (curr.y - prev.y) as f64;

            let hyp = (rx * rx + ry * ry).sqrt();
            if hyp > 0.0 {
                let angle = if ry <= 0.0 {
                    (rx / hyp).acos()
                } else {
                    -(rx / hyp).acos()
                };
                angles.push(angle);
            }
        }
        angles
    }

    fn find_candidates(&self, angles: &[f64], bindings: &[Binding]) -> Vec<Candidate> {
        let mut candidates: Vec<Candidate> = bindings
            .iter()
            .filter_map(|binding| {
                let binding_angles = self.points_to_angles(&binding.points);

                if binding_angles.len() < SHAPE_MIN_SIZE {
                    return None;
                }

                let diff = self.compare_angles_with_offset(angles, &binding_angles);

                if diff > 300.0 {
                    return None;
                }

                Some(Candidate {
                    diff,
                    binding: binding.clone(),
                })
            })
            .collect();

        candidates.sort_by(|a, b| a.diff.partial_cmp(&b.diff).unwrap());

        candidates
    }
}

impl Recognizer for AngleRecognizer {
    fn find_candidate(&self, points: &[Point], bindings: &Vec<Binding>) -> Option<Candidate> {
        let angles = self.points_to_angles(points);

        if angles.len() <= SHAPE_MIN_SIZE {
            return None;
        }

        let candidates = self.find_candidates(&angles, bindings);
        let first = candidates.first()?;

        info::propabilities(&candidates);

        if first.diff >= DIFF_MAX {
            return None;
        }

        match candidates.get(1) {
            Some(second) if second.diff - first.diff <= DIFF_MIN_WITH_SECOND => Some(first.clone()),
            _ => Some(first.clone()),
        }
    }
}

#[cfg(test)]
#[path = "./__angle_recognizer_test.rs"]
mod test;
