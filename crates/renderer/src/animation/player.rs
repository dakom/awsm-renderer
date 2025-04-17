use super::{clip::AnimationClip, data::AnimationData, TransformAnimation};

#[derive(Debug, Clone)]
pub struct AnimationPlayer<T = AnimationData> {
    pub speed: f64,
    pub loop_style: Option<AnimationLoopStyle>,
    // will change with ping-pong as each end is hit
    pub play_direction: AnimationPlayDirection,
    clip: AnimationClip<T>,
    state: AnimationState,
    local_time: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationState {
    Playing,
    Paused,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationLoopStyle {
    Loop,
    PingPong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationPlayDirection {
    Forward,
    Backward,
}

impl<T> AnimationPlayer<T> {
    pub fn new(clip: AnimationClip<T>) -> Self {
        Self {
            speed: 1.0,
            loop_style: None,
            play_direction: AnimationPlayDirection::Forward,
            clip,
            state: AnimationState::Paused,
            local_time: 0.0,
        }
    }

    pub fn update(&mut self, global_time_delta: f64) {
        if self.state != AnimationState::Playing {
            return;
        }

        let local_time_delta = global_time_delta * self.speed;

        match self.play_direction {
            AnimationPlayDirection::Forward => {
                self.local_time += local_time_delta;
                if self.local_time >= self.clip.duration {
                    match self.loop_style {
                        Some(AnimationLoopStyle::Loop) => {
                            self.local_time = self.local_time.rem_euclid(self.clip.duration);
                        }
                        Some(AnimationLoopStyle::PingPong) => {
                            self.play_direction = AnimationPlayDirection::Backward;
                            self.local_time = self.clip.duration;
                        }
                        None => {
                            self.local_time = self.clip.duration;
                            self.state = AnimationState::Ended;
                        }
                    }
                }
            }

            AnimationPlayDirection::Backward => {
                self.local_time -= local_time_delta;
                if self.local_time <= 0.0 {
                    match self.loop_style {
                        Some(AnimationLoopStyle::Loop) => {
                            self.local_time =
                                self.clip.duration - self.local_time.rem_euclid(self.clip.duration);
                        }
                        Some(AnimationLoopStyle::PingPong) => {
                            self.play_direction = AnimationPlayDirection::Forward;
                            self.local_time = 0.0;
                        }
                        None => {
                            self.local_time = 0.0;
                            self.state = AnimationState::Ended;
                        }
                    }
                }
            }
        }
    }
}

impl AnimationPlayer<AnimationData> {
    pub fn sample(&self) -> AnimationData {
        self.clip.sampler.sample(self.local_time)
    }
}
