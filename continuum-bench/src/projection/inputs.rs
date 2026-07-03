//! Load projection inputs from collected bench JSON reports.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::model::FleetProjectionInputs;

/// Load BM-L* achieved rates for fleet projection.
pub fn load_from_dir(
    reports_dir: &Path,
    hardware: &str,
    storage: &str,
    tikv_topology: Option<&str>,
    scylla_topology: Option<&str>,
) -> Result<FleetProjectionInputs> {
    let mut inputs = FleetProjectionInputs {
        hardware: hardware.into(),
        storage: storage.into(),
        tikv_topology: tikv_topology.map(str::to_string),
        scylla_topology: scylla_topology.map(str::to_string),
        ..FleetProjectionInputs::default()
    };

    for entry in std::fs::read_dir(reports_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let v: Value = serde_json::from_str(&text)?;
        if !matches_dimensions(&v, hardware, storage, tikv_topology, scylla_topology) {
            continue;
        }
        merge_report(&mut inputs, &v);
    }

    if inputs.per_shard_ceiling.is_none() && inputs.aggregate_ops_per_sec.is_none() {
        bail!(
            "missing BM-L* or BM-M2 achieved_ops_per_sec for {hardware}/{storage} in {}",
            reports_dir.display()
        );
    }
    Ok(inputs)
}

fn matches_dimensions(
    v: &Value,
    hardware: &str,
    storage: &str,
    tikv_topology: Option<&str>,
    scylla_topology: Option<&str>,
) -> bool {
    let dims = v.get("dimensions").and_then(|d| d.as_object());
    let Some(d) = dims else {
        return false;
    };
    if d.get("hardware").and_then(|h| h.as_str()) != Some(hardware) {
        return false;
    }
    if d.get("storage").and_then(|s| s.as_str()) != Some(storage) {
        return false;
    }
    if let Some(topo) = tikv_topology {
        if d.get("tikv_topology").and_then(|t| t.as_str()) != Some(topo) {
            return false;
        }
    }
    if let Some(topo) = scylla_topology {
        if d.get("scylla_topology").and_then(|t| t.as_str()) != Some(topo) {
            return false;
        }
    }
    // Skip partitioned load-tier reports when projecting hot-stream ceiling.
    if scylla_topology.is_none() && tikv_topology.is_none() {
        if let Some(k) = v.pointer("/metrics/load_partition_count").and_then(|x| x.as_u64()) {
            if k > 1 {
                return false;
            }
        }
    }
    true
}

fn merge_report(inputs: &mut FleetProjectionInputs, v: &Value) {
    let id = v.get("experiment_id").and_then(|e| e.as_str()).unwrap_or("");
    let rate = v
        .pointer("/metrics/achieved_ops_per_sec")
        .and_then(serde_json::Value::as_f64);
    match id {
        "bm-l3" => inputs.per_shard_ceiling = rate.or(inputs.per_shard_ceiling),
        "bm-l2" | "bm-l1" | "bm-l0" if inputs.per_shard_ceiling.is_none() => {
            inputs.per_shard_ceiling = rate;
        }
        "bm-m2" | "bm-m4" => {
            inputs.aggregate_ops_per_sec = rate.or(inputs.aggregate_ops_per_sec);
            inputs.partitions_modeled = v
                .pointer("/metrics/partitions_modeled")
                .or_else(|| v.pointer("/metrics/partition_count"))
                .and_then(serde_json::Value::as_u64)
                .or(inputs.partitions_modeled);
            inputs.clients_modeled = v
                .pointer("/metrics/clients_modeled")
                .or_else(|| v.pointer("/metrics/client_count"))
                .and_then(serde_json::Value::as_u64)
                .or(inputs.clients_modeled);
        }
        _ => {}
    }
    if let Some(n) = v
        .pointer("/dimensions/surreal_instances")
        .and_then(serde_json::Value::as_u64)
    {
        inputs.surreal_instances = u8::try_from(n).unwrap_or(1);
    }
}

/// Write projection JSON to disk.
pub fn write_projection(path: &Path, projection: &super::model::FleetProjection) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(projection)?;
    std::fs::write(path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
