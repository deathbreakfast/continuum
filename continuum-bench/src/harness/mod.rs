//! Benchmark harness: dimensions, hardware capture, backend factory.

pub mod backend;
pub mod dimensions;
pub mod hardware;

pub use backend::{
    build_backend, backend_kind, open_shared_postgres, open_shared_scylla, open_shared_sqlite,
    open_shared_surreal, open_shared_tikv_raw, BackendHandle, BenchBackend, SharedHandle,
};
pub use dimensions::{
    matrix_for_subset, subset_needs_remote_surreal, ComponentHardware, ExperimentId, Hardware,
    MatrixSubset, RunDimensions, Storage, ScyllaTopology, SurrealDeployment,
    surreal_instances_from_env, Telemetry, TikvTopology, Topology,
};
pub use hardware::{capture as capture_hardware, HardwareDetail};
