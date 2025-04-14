use std::cmp::Ordering;

use super::data::Animatable;

#[derive(Debug, Clone)]
pub struct AnimationClip <T> {
    pub name: Option<String>,
    pub duration: f64,
    pub sampler: AnimationSampler<T>, 
}

#[derive(Debug, Clone)]
pub enum AnimationSampler <T> {
    Linear {
        times: Vec<f64>,
        values: Vec<T>,
    },
    Step {
        times: Vec<f64>,
        values: Vec<T>,
    },
    CubicSpline {
        times: Vec<f64>,
        values: Vec<T>,
        in_tangents: Vec<T>,
        out_tangents: Vec<T>,
    },
}

impl <T> AnimationClip<T> {
    pub fn new(name: Option<String>, duration: f64, sampler: AnimationSampler<T>) -> Self {
        Self {
            name,
            duration,
            sampler
        }
    }
}

impl <T: Animatable> AnimationSampler<T> {
    pub fn new_linear(times: Vec<f64>, values: Vec<T>) -> Self {
        Self::Linear {
            times,
            values,
        }
    }

    pub fn new_step(times: Vec<f64>, values: Vec<T>) -> Self {
        Self::Step {
            times,
            values,
        }
    }

    pub fn new_cubic_spline(times: Vec<f64>, values: Vec<T>, in_tangents: Vec<T>, out_tangents: Vec<T>) -> Self {
        Self::CubicSpline {
            times,
            values,
            in_tangents,
            out_tangents,
        }
    }

    pub fn times(&self) -> &[f64] {
        match self {
            AnimationSampler::Linear { times, .. } => times,
            AnimationSampler::Step { times, .. } => times,
            AnimationSampler::CubicSpline { times, .. } => times,
        }
    }

    pub fn sample(&self, time: f64) -> T {
        let bounds = self.binary_search_bounds(time);

        match bounds {
            BinaryBounds::ExactHit(index) => {
                match self {
                    AnimationSampler::Linear { values, .. } => values[index].clone(),
                    AnimationSampler::Step { values, .. } => values[index].clone(),
                    AnimationSampler::CubicSpline { values, .. } => values[index].clone(),
                }
            },
            BinaryBounds::Between(left_index, right_index) => {
                let times = self.times();
                let left_time = times[left_index];
                let right_time = times[right_index];

                match self {
                    AnimationSampler::Linear { values, .. } => {
                        let left_value = &values[left_index];
                        let right_value = &values[right_index];

                        let interpolation_time = (time - left_time) / (right_time - left_time);

                        T::interpolate_linear(left_value, right_value, interpolation_time)
                    },
                    AnimationSampler::Step { values, .. } => {
                        values[left_index].clone()
                    },
                    AnimationSampler::CubicSpline { values, in_tangents, out_tangents, .. } => {
                        let interpolation_time = (time - left_time) / (right_time - left_time);
                        let delta_time = right_time - left_time;
                        let left_value = &values[left_index];
                        let right_value = &values[right_index];
                        let left_tangent = &out_tangents[left_index];
                        let right_tangent = &in_tangents[right_index];

                        T::interpolate_cubic_spline(left_value, left_tangent, right_value, right_tangent, delta_time, interpolation_time)
                    }
                }
            },
        }

    }

    // Returns the index of the keyframe that is closest to the given time
    // BinaryBounds::ExactHit(usize) if the time is exactly on a keyframe
    // BinaryBounds::Middle(usize, usize) if the time is between two keyframes
    fn binary_search_bounds(&self, time: f64) -> BinaryBounds {
        let times = self.times();

        if times.is_empty() {
            panic!("Cannot search an empty times array");
        }

        match times.binary_search_by(|t| t.partial_cmp(&time).unwrap_or(Ordering::Equal)) {
            Ok(i) => BinaryBounds::ExactHit(i),
            Err(i) => {
                if i == 0 {
                    BinaryBounds::Between(0, 1)
                } else if i >= times.len() {
                    // This shouldn't really happen, but just in case, clamp to the end
                    BinaryBounds::ExactHit(times.len() - 1)
                } else {
                    BinaryBounds::Between(i - 1, i)
                }
            }
        }
    }
}

enum BinaryBounds {
    ExactHit(usize),
    Between(usize, usize),
}
