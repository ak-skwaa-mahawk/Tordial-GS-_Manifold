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
    ManifoldTick { timestamp: u64, velocity_vector: Vec<f64> },
    SoilLiquidity { liquidity_coefficient: f64, saturation_delta: f64 },
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

    pub stability_counter: usize,
    pub stability_threshold: usize,
    pub regeneration_amount: f64,
    pub max_compute: f64,
    pub rest_cycles_remaining: usize,

    pub wake_threshold_distance: f64,
    pub wake_threshold_forcing: f64,
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
        }
    }

    pub fn set_target(&mut self, target: Vec<f64>) -> Result<(), &'static str> {
        if target.len() < 2 { return Err("Target requires at least 2 coordinates [theta, phi]."); }
        self.internal_target = Some(vec![Self::wrap_angle(target[0]), Self::wrap_angle(target[1])]);
        self.rest_cycles_remaining = 0;
        Ok(())
    }

    pub fn clear_target(&mut self) {
        self.internal_target = None;
        self.rest_cycles_remaining = 0;
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
        &mut self, state: &[f64], delta: f64, ts: u64
    ) -> Result<(Vec<f64>, bool, DecisionStatus), &'static str> {
        if state.len() < 4 { return Err("4D state required"); }
        let target = match &self.internal_target {
            Some(t) => t,
            None => return Ok((state.to_vec(), false, DecisionStatus::RejectedStagnation)),
        };
        if self.rest_cycles_remaining > 0 {
            self.rest_cycles_remaining -= 1;
            self.log_decision(ts, 0.0, 0.0, 0.0, DecisionStatus::DeepStabilityRest);
            return Ok((state.to_vec(), false, DecisionStatus::DeepStabilityRest));
        }
        if state[3] <= 0.01 {
            self.log_decision(ts, 0.0, 0.0, 0.0, DecisionStatus::RejectedExhaustion);
            return Ok((state.to_vec(), false, DecisionStatus::RejectedExhaustion));
        }
        let dist = self.compute_geodesic_distance(state, target)?;
        if dist < 1e-4 || delta < 5e-4 {
            self.stability_counter += 1;
            if self.stability_counter >= self.stability_threshold {
                self.stability_counter = 0;
                if state[3] >= self.max_compute {
                    self.rest_cycles_remaining = 10;
                    self.log_decision(ts, dist, dist, 0.0, DecisionStatus::DeepStabilityRest);
                    return Ok((state.to_vec(), false, DecisionStatus::DeepStabilityRest));
                }
                let reward = self.regeneration_amount.min(self.max_compute - state[3]);
                let mut s = state.to_vec(); s[3] += reward;
                self.log_decision(ts, dist, dist, reward, DecisionStatus::HomeostaticRegen);
                return Ok((s, true, DecisionStatus::HomeostaticRegen));
            }
            self.log_decision(ts, dist, dist, 0.0, DecisionStatus::RejectedStagnation);
            return Ok((state.to_vec(), false, DecisionStatus::RejectedStagnation));
        } else { self.stability_counter = 0; }

        let mut trial = state.to_vec();
        trial[0] = Self::wrap_angle(trial[0] + delta * trial[0].cos());
        trial[1] = Self::wrap_angle(trial[1] + delta * trial