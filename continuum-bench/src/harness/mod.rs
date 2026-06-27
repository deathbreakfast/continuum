//! Benchmark harness: dimensions, hardware capture, backend factory.

pub mod backend;
pub mod dimensions;
pub mod hardware;

pub use backend::{
    build_backend, backend_kind, open_shared_postgres, open_shared_sqlite, open_shared_surreal,
    BackendHandle, BenchBackend, SharedHandle,
};
pub use dimensions::{
    dev_wsl_matrix, sql_adapter_matrix, ExperimentId, Hardware, MatrixSubset, RunDimensions,
    Storage, Telemetry, Topology,
};
pub use hardware::{capture as capture_hardware, HardwareDetail};
