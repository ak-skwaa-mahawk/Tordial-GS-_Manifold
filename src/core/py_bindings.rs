use pyo3::prelude::*;
use std::sync::{Arc, Mutex};
use crate::core::actor::SovereignEconomicActor;
use crate::core::substrate_bus::PySubstrateMeshBus;

#[pyclass]
pub struct PyCombiner {
    inner: Arc<Mutex<SovereignEconomicActor>>,
    pub actor_id: u64,
}

#[pymethods]
impl PyCombiner {
    #[new]
    pub fn new(actor_id: u64, initial_forcing: f64) -> Self {
        PyCombiner {
            inner: Arc::new(Mutex::new(SovereignEconomicActor::new(actor_id, initial_forcing))),
            actor_id,
        }
    }

    pub fn register_to_bus(&mut self, bus: &PySubstrateMeshBus) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.register_bus(Arc::new(Mutex::new(PySubstrateMeshBus::new(bus.base_forcing_scale))));
        }
    }

    pub fn configure_homeostasis(&mut self, threshold: usize, amount: f64, max_cap: f64) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.stability_threshold = threshold;
            actor.regeneration_amount = amount;
            actor.max_compute = max_cap;
        }
    }

    pub fn configure_sensory_interrupt(&mut self, distance_threshold: f64, forcing_threshold: f64) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.wake_threshold_distance = distance_threshold;
            actor.wake_threshold_forcing = forcing_threshold;
        }
    }

    pub fn get_alpha_forcing(&self) -> f64 {
        self.inner.lock().map(|a| a.alpha_forcing).unwrap_or(0.0)
    }

    pub fn is_in_deep_rest(&self) -> bool {
        self.inner.lock().map(|a| a.rest_cycles_remaining > 0).unwrap_or(false)
    }

    pub fn get_distance_to_target(&self, state: Vec<f64>) -> PyResult<f64> {
        if let Ok(actor) = self.inner.lock() {
            if let Some(target) = &actor.internal_target {
                return actor.compute_geodesic_distance(&state, target)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e));
            }
        }
        Ok(0.0)
    }

    pub fn get_stability_status(&self) -> (usize, usize, f64, f64, usize) {
        if let Ok(actor) = self.inner.lock() {
            (
                actor.stability_counter,
                actor.stability_threshold,
                actor.regeneration_amount,
                actor.max_compute,
                actor.rest_cycles_remaining,
            )
        } else {
            (0, 0, 0.0, 0.0, 0)
        }
    }

    pub fn set_internal_target(&mut self, target: Vec<f64>) -> PyResult<()> {
        if let Ok(mut actor) = self.inner.lock() {
            actor.set_target(target).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err("Lock poisoned"))
        }
    }

    pub fn clear_internal_target(&mut self) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.clear_target();
        }
    }

    pub fn push_manifold_tick(&mut self, timestamp: u64, velocity_vector: Vec<f64>) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.push_update(crate::core::actor::SubstrateUpdate::ManifoldTick {
                timestamp,
                velocity_vector,
            });
        }
    }

    pub fn push_soil_liquidity(&mut self, liquidity_coefficient: f64, saturation_delta: f64) {
        if let Ok(mut actor) = self.inner.lock() {
            actor.push_update(crate::core::actor::SubstrateUpdate::SoilLiquidity {
                liquidity_coefficient,
                saturation_delta,
            });
        }
    }

    pub fn run_autonomous_session(
        &mut self,
        mut current_state: Vec<f64>,
        total_cycles: usize,
        delta_jiggle: f64,
        start_time: u64,
    ) -> PyResult<(Vec<f64>, usize)> {
        if let Ok(mut actor) = self.inner.lock() {
            match actor.run_autonomous_session(&mut current_state, total_cycles, delta_jiggle, start_time) {
                Ok(cycles) => Ok((current_state, cycles)),
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
            }
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err("Lock poisoned"))
        }
    }

    pub fn get_decision_history(&self) -> PyResult<Vec<(u64, f64, f64, f64, u32)>> {
        if let Ok(actor) = self.inner.lock() {
            Ok(actor.decision_ledger.iter()
                .map(|r| (r.timestamp, r.initial_distance, r.trial_distance, r.utility_gain, r.status as u32))
                .collect())
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err("Lock poisoned"))
        }
    }
}