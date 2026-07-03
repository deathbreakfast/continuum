//! 1B/s fleet projection from collected Continuum BM-L* report JSONs.
//!
//! **Internal — performance engineers.** Reads `achieved_ops_per_sec` from load-tier
//! reports and estimates partition count, node count, and $/M ops toward a 1B/s target.

mod inputs;
mod model;
mod render;
mod scaling;

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Build and print a fleet projection from report JSONs in `reports_dir`.
    pub fn project_fleet(
    hardware: &str,
    storage: &str,
    tikv_topology: Option<&str>,
    scylla_topology: Option<&str>,
    reports_dir: &Path,
    out: Option<PathBuf>,
) -> Result<()> {
    let mut inputs = inputs::load_from_dir(
        reports_dir,
        hardware,
        storage,
        tikv_topology,
        scylla_topology,
    )?;
    if let Some(hw) = crate::harness::Hardware::from_slug(hardware) {
        inputs.hourly_usd = hw.hourly_usd();
    }
    let projection = model::compute(&inputs);
    let topo_suffix = tikv_topology
        .or(scylla_topology)
        .unwrap_or("any");
    let out_path = out.unwrap_or_else(|| {
        reports_dir.join(format!("projection-{hardware}-{storage}-{topo_suffix}.json"))
    });
    inputs::write_projection(&out_path, &projection)?;
    println!("wrote {}", out_path.display());
    println!("{}", render::render_markdown(&projection));
    Ok(())
}

/// Print storage-node scaling curve from peak BM-M4 reports across topologies.
pub fn project_scaling_curve(
    hardware: &str,
    storage: &str,
    reports_dir: &Path,
    out: Option<PathBuf>,
) -> Result<()> {
    let curve = scaling::load_scaling_curve(reports_dir, hardware, storage)?;
    let out_path = out.unwrap_or_else(|| {
        reports_dir.join(format!("scaling-curve-{hardware}-{storage}.json"))
    });
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&curve)?;
    std::fs::write(&out_path, json)?;
    println!("wrote {}", out_path.display());
    println!("{}", scaling::render_scaling_markdown(&curve));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::model::{compute, FleetProjectionInputs};

    #[test]
    fn projection_nodes_for_1b_target() {
        let inputs = FleetProjectionInputs {
            hardware: "dev-wsl".into(),
            storage: "surreal-tikv".into(),
            tikv_topology: Some("tikv-ha-3".into()),
            scylla_topology: None,
            per_shard_ceiling: Some(10_000.0),
            hourly_usd: 0.0416,
            surreal_instances: 1,
            ..FleetProjectionInputs::default()
        };
        let p = compute(&inputs);
        assert_eq!(p.partitions_for_1e9, Some(100_000));
        assert!(p.nodes_required.unwrap_or(0) >= 100_000);
    }
}
