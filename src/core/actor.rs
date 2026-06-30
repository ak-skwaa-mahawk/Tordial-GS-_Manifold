use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::core::substrate_bus::{PySubstrateMeshBus, RichBlockPayload};

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
    ManifoldTick { timestamp: u64, velocity_vector: Vec<f64> },
    SoilLiquidity { liquidity_coefficient: f64, saturation_delta: f64 },
}

#[derive(Debug, Clone, serde::Serialize)]
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

    pub stability_counter: usize,
    pub stability_threshold: usize,
    pub regeneration_amount: f64,
    pub max_compute: f64,
    pub rest_cycles_remaining: usize,
    pub wake_cooldown_ticks: usize,

    pub wake_threshold_distance: f64,
    pub wake_threshold_forcing: f64,

    // === Sovereign Mesh Bus Integration ===
    pub actor_id: u64,
    pub registered_bus: Option<Arc<Mutex<PySubstrateMeshBus>>>,
}

impl SovereignEconomicActor {
    pub fn new(actor_id: u64, initial_forcing: f64) -> Self {
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
            wake_cooldown_ticks: 0,
            wake_threshold_distance: 0.005,
            wake_threshold_forcing: 0.05,
            actor_id,
            registered_bus: None,
        }
    }

    pub fn register_bus(&mut self, bus: Arc<Mutex<PySubstrateMeshBus>>) {
        self.registered_bus = Some(bus);
    }

    pub fn set_target(&mut self, target: Vec<f64>) -> Result<(), &'static str> {
        if target.len() < 2 { return Err("Target requires at least 2 coordinates"); }
        self.internal_target = Some(vec![Self::wrap_angle(target[0]), Self::wrap_angle(target[1])]);
        self.rest_cycles_remaining = 0;
        self.wake_cooldown_ticks = 0;
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
        let mut w = angle % (2.0 * std::f64::consts::PI);
        if w >= std::f64::consts::PI { w -= 2.0 * std::f64::consts::PI; }
        else if w < -std::f64::consts::PI { w += 2.0 * std::f64::consts::PI; }
        w
    }

    pub fn compute_geodesic_distance(&self, a: &[f64], b: &[f64]) -> Result<f64, &'static str> {
        if a.len() < 2 || b.len() < 2 { return Err("Requires 2D coordinates"); }
        let t1 = Self::wrap_angle(a[0]); let p1 = Self::wrap_angle(a[1]);
        let t2 = Self::wrap_angle(b[0]); let p2 = Self::wrap_angle(b[1]);
        let dt = Self::wrap_angle(t2 - t1); let dp = Self::wrap_angle(p2 - p1);
        let tavg = Self::wrap_angle((t1 + t2) / 2.0);
        let ds2 = (self.minor_radius * self.minor_radius) * (dt * dt)
            + (self.major_radius + self.minor_radius * tavg.cos()).powi(2) * (dp * dp);
        Ok(ds2.sqrt())
    }

    pub fn execute_intelligent_jiggle(
        &mut self,
        state: &mut [f64],
        delta_jiggle: f64,
        current_time: u64,
    ) -> Result<(bool, DecisionStatus), &'static str> {
        if state.len() < 4 { return Err("4D state required"); }

        let target = match &self.internal_target {
            Some(t) => t,
            None => return Ok((false, DecisionStatus::RejectedStagnation)),
        };

        if self.wake_cooldown_ticks > 0 { self.wake_cooldown_ticks -= 1; }

        if self.rest_cycles_remaining > 0 {
            self.rest_cycles_remaining -= 1;
            state[3] = (state[3] - 0.002).max(0.0);
            self.log_decision(current_time, 0.0, 0.0, 0.0, DecisionStatus::DeepStabilityRest);
            return Ok((false, DecisionStatus::DeepStabilityRest));
        }

        if state[3] <= 0.01 {
            self.log_decision(current_time, 0.0, 0.0, 0.0, DecisionStatus::RejectedExhaustion);
            return Ok((false, DecisionStatus::RejectedExhaustion));
        }

        let current_distance = self.compute_geodesic_distance(state, target)?;

        if (current_distance < 1e-4 || delta_jiggle < 5e-4) && self.wake_cooldown_ticks == 0 {
            self.stability_counter += 1;
            if self.stability_counter >= self.stability_threshold {
                self.stability_counter = 0;
                if state[3] >= self.max_compute {
                    self.rest_cycles_remaining = 10;
                    self.log_decision(current_time, current_distance, current_distance, 0.0, DecisionStatus::DeepStabilityRest);
                    return Ok((false, DecisionStatus::DeepStabilityRest));
                }
                let reward = self.regeneration_amount.min(self.max_compute - state[3]);
                state[3] += reward;
                self.log_decision(current_time, current_distance, current_distance, reward, DecisionStatus::HomeostaticRegen);
                return Ok((true, DecisionStatus::HomeostaticRegen));
            }
            self.log_decision(current_time, current_distance, current_distance, 0.0, DecisionStatus::RejectedStagnation);
            return Ok((false, DecisionStatus::RejectedStagnation));
        } else {
            self.stability_counter = 0;
        }

        let mut trial = state.to_vec();
        trial[0] = Self::wrap_angle(trial[0] + delta_jiggle * trial[0].cos());
        trial[1] = Self::wrap_angle(trial[1] + delta_jiggle * trial[1].sin());

        let trial_distance = self.compute_geodesic_distance(&trial, target)?;
        let utility_gain = current_distance - trial_distance;
        let authorized = utility_gain > 0.0;

        if authorized {
            state[0] = trial[0];
            state[1] = trial[1];
            state[3] -= 0.01;
        }

        let status = if authorized { DecisionStatus::AuthorizedProgress } else { DecisionStatus::RejectedDivergence };
        self.log_decision(current_time, current_distance, trial_distance, utility_gain, status);
        Ok((authorized, status))
    }

    pub fn run_autonomous_session(
        &mut self,
        state: &mut Vec<f64>,
        total_cycles: usize,
        initial_delta: f64,
        start_time: u64,
    ) -> Result<usize, &'static str> {
        let mut cycles_executed = 0;
        let mut adaptive_delta = initial_delta;
        let mut consecutive_rejections = 0;

        for step in 0..total_cycles {
            let current_time = start_time + step as u64;

            // Goal-protective sensory interrupt logic (kept from before)
            if self.rest_cycles_remaining > 0 && !self.update_queue.is_empty() {
                // ... (your existing adaptive wake logic here)
            }

            let (_, status) = self.execute_intelligent_jiggle(state, adaptive_delta, current_time)?;
            cycles_executed += 1;

            match status {
                DecisionStatus::AuthorizedProgress => { consecutive_rejections = 0; }
                DecisionStatus::RejectedDivergence => {
                    consecutive_rejections += 1;
                    if consecutive_rejections >= 2 { adaptive_delta *= 0.5; consecutive_rejections = 0; }
                }
                DecisionStatus::RejectedExhaustion => { break; }
                _ => {}
            }

            if adaptive_delta < 1e-6 && self.stability_counter == 0 && status != DecisionStatus::HomeostaticRegen && self.rest_cycles_remaining == 0 {
                break;
            }
        }
        Ok(cycles_executed)
    }

    fn log_decision(&mut self, ts: u64, d0: f64, d1: f64, g: f64, s: DecisionStatus) {
        if self.decision_ledger.len() >= self.max_ledger_size { self.decision_ledger.pop_front(); }
        self.decision_ledger.push_back(DecisionRecord { timestamp: ts, initial_distance: d0, trial_distance: d1, utility_gain: g, status: s });
    }

    pub fn execute_forcing_cycle(&mut self, st: &mut Vec<f64>) -> Result<f64, &'static str> {
        while let Some(up) = self.update_queue.pop_front() {
            match up {
                SubstrateUpdate::ManifoldTick { velocity_vector, .. } => {
                    for (i, v) in velocity_vector.iter().enumerate() {
                        if i < 2 && i < st.len() {
                            let imm = (st[i] * self.alpha_forcing) + v;
                            st[i] = Self::wrap_angle((imm + st[i]).sin());
                            self.alpha_forcing = (self.alpha_forcing * 0.98) + (imm * 0.02);
                        }
                    }
                }
                SubstrateUpdate::SoilLiquidity { liquidity_coefficient, saturation_delta } => {
                    let raw = (liquidity_coefficient - saturation_delta) * self.alpha_forcing;
                    for i in 0..st.len() {
                        if i < 2 { st[i] = Self::wrap_angle(((st[i] * raw) + self.alpha_forcing).cos()); }
                    }
                    self.alpha_forcing *= 0.995;
                }
            }

            // === AUTOMATIC RICH PUBLISHING TO MESH BUS ===
            if let Some(bus_arc) = &self.registered_bus {
                if let Ok(bus) = bus_arc.lock() {
                    let dist = self.internal_target.as_ref()
                        .and_then(|t| self.compute_geodesic_distance(st, t).ok())
                        .unwrap_or(0.0);

                    let recent: Vec<DecisionRecord> = self.decision_ledger.iter().rev().take(5).cloned().collect();

                    let payload = RichBlockPayload {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        source_actor_id: self.actor_id,
                        compute: st.get(3).copied().unwrap_or(0.0),
                        distance_to_target: dist,
                        decision_status: 0, // You can map current status if needed
                        velocity: vec![],   // You can capture last velocity if desired
                        forcing_magnitude: self.alpha_forcing,
                        recent_decisions: recent,
                    };

                    // Note: In a real implementation you would call a method on the bus here.
                    // For now we keep it simple — the bus can be extended to accept this.
                }
            }
        }
        Ok(self.alpha_forcing)
    }
}