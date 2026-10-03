pub mod card;
pub mod combat;
pub mod creature;
pub mod damage;
pub mod enemy;
pub mod obs;
pub mod player;
pub mod power;
pub mod rng;

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn rusted_spire(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(m)
}
