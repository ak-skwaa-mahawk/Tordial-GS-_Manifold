use std::collections::VecDeque;

/// Represents the strict, sequential updates coming from the Protobuf stream.
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

pub struct NonAssociativeCombiner {
    /// Strict FIFO queue to guarantee evaluation order regardless of arrival bursts
    update_queue: VecDeque<SubstrateUpdate>,
    /// Base forcing coefficient representing the active manifold state
    pub alpha_forcing: f64,
}

impl NonAssociativeCombiner {
    pub fn new(initial_forcing: f64) -> Self {
        Self {
            update_queue: VecDeque::new(),
            alpha_forcing: initial_forcing,
        }
    }

    /// Safely push a verified Protobuf-derived update into the strict serialization pipeline.
    pub fn push_update(&mut self, update: SubstrateUpdate) {
        self.update_queue.push_back(update);
    }

    /// Non-Associative Left-Associative Forcing Reduction Engine.
    /// Uses explicit grouping: (((A ⊗ B) ⊗ C) ⊗ D) to neutralize arithmetic drift.
    pub fn execute_forcing_cycle(&mut self, current_state: &mut Vec<f64>) -> Result<f64, &'static str> {
        if self.update_queue.is_empty() {
            return Ok(self.alpha_forcing);
        }

        while let Some(update) = self.update_queue.pop_front() {
            match update {
                SubstrateUpdate::ManifoldTick { velocity_vector, .. } => {
                    for (i, val) in velocity_vector.iter().enumerate() {
                        if i < current_state.len() {
                            // Enforce strict non-associative step:
                            // State combining relies heavily on the sequence order
                            let immediate_forcing = (current_state[i] * self.alpha_forcing) + val;
                            current_state[i] = (immediate_forcing + current_state[i]).sin();
                            self.alpha_forcing = (self.alpha_forcing * 0.98) + (immediate_forcing * 0.02);
                        }
                    }
                }
                SubstrateUpdate::SoilLiquidity { liquidity_coefficient, saturation_delta } => {
                    // Enforce localized non-associative evaluation mapping
                    let raw_liquidity_forcing = (liquidity_coefficient - saturation_delta) * self.alpha_forcing;
                    
                    for state_val in current_state.iter_mut() {
                        *state_val = ((*state_val * raw_liquidity_forcing) + self.alpha_forcing).cos();
                    }
                    
                    // Force state transition on the baseline accumulator
                    self.alpha_forcing = (self.alpha_forcing + raw_liquidity_forcing) * 0.995;
                }
            }
        }

        // Return the hardened forcing accumulator to pass to the SovereignOperator
        Ok(self.alpha_forcing)
    }
}
