use pyo3::prelude::*;
use crate::core::combiner::{NonAssociativeCombiner, SubstrateUpdate};

#[pyclass]
pub struct PyCombiner {
    inner: NonAssociativeCombiner,
}

#[pymethods]
impl PyCombiner {
    #[new]
    fn new(initial_forcing: f64) -> Self {
        PyCombiner {
            inner: NonAssociativeCombiner::new(initial_forcing),
        }
    }

    fn push_manifold_tick(&mut self, timestamp: u64, velocity_vector: Vec<f64>) {
        self.inner.push_update(SubstrateUpdate::ManifoldTick {
            timestamp,
            velocity_vector,
        });
    }

    fn push_soil_liquidity(&mut self, liquidity_coefficient: f64, saturation_delta: f64) {
        self.inner.push_update(SubstrateUpdate::SoilLiquidity {
            liquidity_coefficient,
            saturation_delta,
        });
    }

    fn process_cycle(&mut self, mut current_state: Vec<f64>) -> PyResult<(Vec<f64>, f64)> {
        match self.inner.execute_forcing_cycle(&mut current_state) {
            Ok(forcing) => Ok((current_state, forcing)),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
        }
    }
}
