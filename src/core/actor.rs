// src/core/actor.rs
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStatus {
    AuthorizedProgress = 0,
    RejectedDivergence = 1,
    RejectedStagnation = 2,
    RejectedExhaustion = 3,
    HomeostaticRegen   = 4,
    DeepStabilityRest  = 5,
}

#[derive(Debug, Clone)]
pub enum SubstrateUpdate {
    ManifoldTick {
        timestamp: u64,
        velocity_vector: Vec<f64>,
    },
    SoilLiquidity {
        liquidity_coefficient: f64,
        saturation_delta: f64,
    },
}

#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub timestamp: u64,
    pub initial_distance: f64,
    pub trial_distance: f64,
    pub utility_gain: f64,
    pub status: DecisionStatus,
}

pub struct SovereignEconomicActor {
    update_queue: VecDeque<SubstrateUpdate>,
    pub alpha_forcing: f64,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub internal_target: Option<Vec<f64>>,
    pub decision_ledger: VecDeque<DecisionRecord>,
    pub max_ledger_size: usize,

    // Homeostatic Tracking Primitives
    pub stability_counter: usize,
    pub stability_threshold: usize,
    pub regeneration_amount: f64,
    pub max_compute: f64,
    pub rest_cycles_remaining: usize,

    // Adaptive Nervous System Tolerances
    pub wake_threshold_distance: f64,
    pub wake_threshold_forcing: f64,
    pub wake_cooldown_ticks: usize,
}

impl SovereignEconomicActor {
    pub fn new(initial_forcing: f64) -> Self {
        Self {
            update_queue: VecDeque::new(),
            alpha_forcing: initial_forcing,
            major_radius: 2.0,
            minor_radius: 0.5,
            internal_target: None,
            decision_ledger: VecDeque::with_capacity(500),
            max_ledger_size: 500,

            stability_counter: 0,
            stability_threshold: 3,
            regeneration_amount: 0.04,
            max_compute: 1.0,
            rest_cycles_remaining: 0,

            wake_threshold_distance: 0.005,
            wake_threshold_forcing: 0.05,
            wake_cooldown_ticks: 0,
        }
    }

    pub fn set_target(&mut self, target: Vec<f64>) -> Result<(), &'static str> {
        if target.len() < 2 {
            return Err("Target requires at least 2 spatial coordinates [theta, phi].");
        }
        self.internal_target = Some(vec![Self::wrap_angle(target[0]), Self::wrap_angle(target[1])]);
        self.rest_cycles_remaining = 0;
        self.wake_cooldown_ticks = 5;
        Ok(())
    }

    pub fn clear_target(&mut self) {
        self.internal_target = None;
        self.rest_cycles_remaining = 0;
        self.wake_cooldown_ticks = 0;
    }

    pub fn push_update(&mut self, update: SubstrateUpdate) {
        self.update_queue.push_back(update);
    }

    fn wrap_angle(angle: f64) -> f64 {
        let mut wrapped = angle % (2.0 * std::f64::consts::PI);
        if wrapped >= std::f64::consts::PI {
            wrapped -= 2.0 * std::f64::consts::PI;
        } else if wrapped < -std::f64::consts::PI {
            wrapped += 2.0 * std::f64::consts::PI;
        }
        wrapped
    }

    pub fn compute_geodesic_distance(&self, state_a: &[f64], state_b: &[f64]) -> Result<f64, &'static str> {
        if state_a.len() < 2 || state_b.len() < 2 {
            return Err("Geodesic tracking requires at least 2 geometric coordinate dimensions.");
        }

        let theta_1 = Self::wrap_angle(state_a[0]);
        let phi_1   = Self::wrap_angle(state_a[1]);
        let theta_2 = Self::wrap_angle(state_b[0]);
        let phi_2   = Self::wrap_angle(state_b[1]);

        let d_theta = Self::wrap_angle(theta_2 - theta_1);
        let d_phi   = Self::wrap_angle(phi_2 - phi_1);
        let theta_avg = Self::wrap_angle((theta_1 + theta_2) / 2.0);

        let ds_squared = (self.minor_radius * self.minor_radius) * (d_theta * d_theta)
            + (self.major_radius + self.minor_radius * theta_avg.cos()).powi(2) * (d_phi * d_phi);

        Ok(ds_squared.sqrt())
    }

    pub fn execute_intelligent_jiggle(
        &mut self,
        state: &[f64],
        delta_jiggle: f64,
        current_time: u64,
    ) -> Result<(Vec<f64>, bool, DecisionStatus), &'static str> {
        if state.len() < 4 {
            return Err("Sovereign execution requires a 4D State Space [theta, phi, liquidity, compute].");
        }

        let target = match &self.internal_target {
            Some(t) => t,
            None => return Ok((state.to_vec(), false, DecisionStatus::RejectedStagnation)),
        };

        if self.wake_cooldown_ticks > 0 {
            self.wake_cooldown_ticks -= 1;
        }

        if self.rest_cycles_remaining > 0 {
            self.rest_cycles_remaining -= 1;
            self.log_decision(current_time, 0.0, 0.0, 0.0, DecisionStatus::DeepStabilityRest);
            return Ok((state.to_vec(), false, DecisionStatus::DeepStabilityRest));
        }

        if state[3] <= 0.01 {
            self.log_decision(current_time, 0.0, 0.0, 0.0, DecisionStatus::RejectedExhaustion);
            return Ok((state.to_vec(), false, DecisionStatus::RejectedExhaustion));
        }

        let current_distance = self.compute_geodesic_distance(state, target)?;

        if (current_distance < 1e-4 || delta_jiggle < 5e-4) && self.wake_cooldown_ticks == 0 {
            self.stability_counter += 1;

            if self.stability_counter >= self.stability_threshold {
                self.stability_counter = 0;

                if state[3] >= self.max_compute {
                    self.rest_cycles_remaining = 10;
                    self.log_decision(current_time, current_distance, current_distance, 0.0, DecisionStatus::DeepStabilityRest);
                    return Ok((state.to_vec(), false, DecisionStatus::DeepStabilityRest));
                }

                let actual_reward = self.regeneration_amount.min(self.max_compute - state[3]);
                let mut stable_state = state.to_vec();
                stable_state[3] += actual_reward;

                self.log_decision(current_time, current_distance, current_distance, actual_reward, DecisionStatus::HomeostaticRegen);
                return Ok((stable_state, true, DecisionStatus::HomeostaticRegen));
            }

            self.log_decision(current_time, current_distance, current_distance, 0.0, DecisionStatus::RejectedStagnation);
            return Ok((state.to_vec(), false, DecisionStatus::RejectedStagnation));
        } else {
            self.stability_counter = 0;
        }

        let mut trial_state = state.to_vec();
        trial_state[0] = Self::wrap_angle(trial_state[0] + delta_jiggle * trial_state[0].cos());
        trial_state[1] = Self::wrap_angle(trial_state[1] + delta_jiggle * trial_state[1].sin());

        let trial_distance = self.compute_geodesic_distance(&trial_state, target)?;
        let utility_gain = current_distance - trial_distance;

        let authorized = utility_gain > 0.0;
        let final_state = if authorized {
            let mut next = trial_state;
            next[3] -= 0.01;
            next
        } else {
            state.to_vec()
        };

        let status = if authorized {
            DecisionStatus::AuthorizedProgress
        } else {
            DecisionStatus::RejectedDivergence
        };

        self.log_decision(current_time, current_distance, trial_distance, utility_gain, status);
        Ok((final_state, authorized, status))
    }

    pub fn run_autonomous_session(
        &mut self,
        state: &mut Vec<f64>,
        total_cycles: usize,
        initial_delta: f64,
        start_time: u64,
    ) -> Result<(Vec<f64>, usize), &'static str> {
        let mut cycles_executed = 0;
        let mut adaptive_delta = initial_delta;
        let mut consecutive_rejections = 0;

        for step in 0..total_cycles {
            let current_time = start_time + step as u64;

            // === Sovereign Adaptive Sensory Interrupt Layer ===
            if self.rest_cycles_remaining > 0 && !self.update_queue.is_empty() {
                let initial_forcing = self.alpha_forcing;
                let pre_dist = self.internal_target.as_ref()
                    .and_then(|t| self.compute_geodesic_distance(state, t).ok())
                    .unwrap_or(0.0);

                let _ = self.execute_forcing_cycle(state)?;

                let post_dist = self.internal_target.as_ref()
                    .and_then(|t| self.compute_geodesic_distance(state, t).ok())
                    .unwrap_or(0.0);

                let distance_delta = post_dist - pre_dist;
                let is_detrimental_drift = distance_delta > 0.0;

                let compute_reserve_ratio = state[3] / self.max_compute;
                let adaptive_sensitivity_scalar = 0.2 + 0.8 * compute_reserve_ratio;
                
                let local_distance_gate = self.wake_threshold_distance * adaptive_sensitivity_scalar;
                let local_forcing_gate = self.wake_threshold_forcing * adaptive_sensitivity_scalar;

                let forcing_delta = (self.alpha_forcing - initial_forcing).abs();

                // COGNITIVE FIX: Forcing perturbations are bypassed completely if the spatial result is constructive
                let trip_alarm = is_detrimental_drift && (distance_delta > local_distance_gate || forcing_delta > local_forcing_gate);

                if trip_alarm {
                    self.rest_cycles_remaining = 0;
                    self.stability_counter = 0;
                    self.wake_cooldown_ticks = 5;
                    adaptive_delta = initial_delta;
                }
            }

            let (next_state, _authorized, status) =
                self.execute_intelligent_jiggle(state, adaptive_delta, current_time)?;

            *state = next_state;
            cycles_executed += 1;

            match status {
                DecisionStatus::AuthorizedProgress => {
                    consecutive_rejections = 0;
                }
                DecisionStatus::RejectedDivergence => {
                    consecutive_rejections += 1;
                    if consecutive_rejections >= 2 {
                        adaptive_delta *= 0.5;
                        consecutive_rejections = 0;
                    }
                }
                DecisionStatus::RejectedExhaustion => {
                    break;
                }
                _ => {}
            }

            if adaptive_delta < 1e-6
                && self.stability_counter == 0
                && status != DecisionStatus::HomeostaticRegen
                && self.rest_cycles_remaining == 0
            {
                break;
            }
        }

        Ok((state.clone(), cycles_executed))
    }

    pub fn generate_summary_metrics(&self) -> (usize, usize, usize, usize, usize, f64) {
        let mut moves = 0;
        let mut rejects = 0;
        let mut holds = 0;
        let mut regens = 0;
        let mut rests = 0;
        let mut harvested = 0.0;

        for record in self.decision_ledger.iter() {
            match record.status {
                DecisionStatus::AuthorizedProgress => moves += 1,
                DecisionStatus::RejectedDivergence | DecisionStatus::RejectedExhaustion => rejects += 1,
                DecisionStatus::RejectedStagnation => holds += 1,
                DecisionStatus::HomeostaticRegen => {
                    regens += 1;
                    harvested += record.utility_gain;
                }
                DecisionStatus::DeepStabilityRest => rests += 1,
            }
        }
        (moves, rejects, holds, regens, rests, harvested)
    }

    fn log_decision(&mut self, timestamp: u64, init_d: f64, trial_d: f64, gain: f64, status: DecisionStatus) {
        let record = DecisionRecord {
            timestamp,
            initial_distance: init_d,
            trial_distance: trial_d,
            utility_gain: gain,
            status,
        };
        if self.decision_ledger.len() >= self.max_ledger_size {
            self.decision_ledger.pop_front();
        }
        self.decision_ledger.push_back(record);
    }

    pub fn execute_forcing_cycle(&mut self, current_state: &mut Vec<f64>) -> Result<f64, &'static str> {
        if self.update_queue.is_empty() {
            return Ok(self.alpha_forcing);
        }

        while let Some(update) = self.update_queue.pop_front() {
            match update {
                SubstrateUpdate::ManifoldTick { velocity_vector, .. } => {
                    for (i, val) in velocity_vector.iter().enumerate() {
                        if i < current_state.len() && i < 2 {
                            let immediate_forcing = (current_state[i] * self.alpha_forcing) + val;
                            current_state[i] = Self::wrap_angle((immediate_forcing + current_state[i]).sin());
                            self.alpha_forcing = (self.alpha_forcing * 0.98) + (immediate_forcing * 0.02);
                        }
                    }
                }
                SubstrateUpdate::SoilLiquidity { liquidity_coefficient, saturation_delta } => {
                    let raw_liquidity_forcing = (liquidity_coefficient - saturation_delta) * self.alpha_forcing;
                    for i in 0..current_state.len() {
                        if i < 2 {
                            current_state[i] = Self::wrap_angle(((current_state[i] * raw_liquidity_forcing) + self.alpha_forcing).cos());
                        }
                    }
                    self.alpha_forcing = self.alpha_forcing * 0.995;
                }
            }
        }
        Ok(self.alpha_forcing)
    }
}
EOF
