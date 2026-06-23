use pyo3::prelude::*;
use crate::core::actor::{SovereignEconomicActor, SubstrateUpdate};

#[pyclass]
pub struct PyCombiner { inner: SovereignEconomicActor }

#[pymethods]
impl PyCombiner {
    #[new] pub fn new(f: f64) -> Self { PyCombiner { inner: SovereignEconomicActor::new(f) } }

    pub fn configure_homeostasis(&mut self, th: usize, amt: f64, cap: f64) {
        self.inner.stability_threshold = th; self.inner.regeneration_amount = amt; self.inner.max_compute = cap;
    }
    pub fn configure_sensory_interrupt(&mut self, d_th: f64, f_th: f64) {
        self.inner.wake_threshold_distance = d_th; self.inner.wake_threshold_forcing = f_th;
    }

    pub fn get_alpha_forcing(&self) -> f64 { self.inner.alpha_forcing }
    pub fn is_in_deep_rest(&self) -> bool { self.inner.rest_cycles_remaining > 0 }
    pub fn get_distance_to_target(&self, state: Vec<f64>) -> PyResult<f64> {
        match &self.inner.internal_target {
            Some(t) => self.inner.compute_geodesic_distance(&state, t).map_err(|e| pyo3::exceptions::PyValueError::new_err(e)),
            None => Ok(0.0),
        }
    }
    pub fn get_stability_status(&self) -> (usize, usize, f64, f64, usize) {
        (self.inner.stability_counter, self.inner.stability_threshold, self.inner.regeneration_amount,
         self.inner.max_compute, self.inner.rest_cycles_remaining)
    }
    pub fn get_session_summary(&self) -> PyResult<(usize, usize, usize, usize, usize, f64)> {
        Ok(self.inner.generate_summary_metrics())
    }

    pub fn set_internal_target(&mut self, t: Vec<f64>) -> PyResult<()> {
        self.inner.set_target(t).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    }
    pub fn clear_internal_target(&mut self) { self.inner.clear_target(); }

    pub fn push_manifold_tick(&mut self, ts: u64, v: Vec<f64>) {
        self.inner.push_update(SubstrateUpdate::ManifoldTick { timestamp: ts, velocity_vector: v });
    }
    pub fn push_soil_liquidity(&mut self, lc: f64, sd: f64) {
        self.inner.push_update(SubstrateUpdate::SoilLiquidity { liquidity_coefficient: lc, saturation_delta: sd });
    }

    pub fn native_geodesic_distance(&self, a: Vec<f64>, b: Vec<f64>) -> PyResult<f64> {
        self.inner.compute_geodesic_distance(&a, &b).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    }

    pub fn trigger_autonomous_jiggle(&mut self, st: Vec<f64>, d: f64, ts: u64) -> PyResult<(Vec<f64>, bool)> {
        match self.inner.execute_intelligent_jiggle(&st, d, ts) {
            Ok((next, exec, _)) => Ok((next, exec)),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
        }
    }

    pub fn run_autonomous_session(&mut self, mut st: Vec<f64>, tot: usize, d: f64, ts: u64) -> PyResult<(Vec<f64>, usize)> {
        match self.inner.run_autonomous_session(&mut st, tot, d, ts) {
            Ok((final_st, cyc)) => Ok((final_st, cyc)),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
        }
    }

    pub fn get_decision_history(&self) -> PyResult<Vec<(u64, f64, f64, f64, u32)>> {
        Ok(self.inner.decision_ledger.iter().map(|r| (r.timestamp, r.initial_distance, r.trial_distance, r.utility_gain, r.status as u32)).collect())
    }

    pub fn process_cycle(&mut self, mut st: Vec<f64>) -> PyResult<(Vec<f64>, f64)> {
        match self.inner.execute_forcing_cycle(&mut st) {
            Ok(f) => Ok((st, f)),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
        }
    }
}