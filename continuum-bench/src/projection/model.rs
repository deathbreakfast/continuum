//! Fleet projection model for 1B/s decomposition.

use serde::Serialize;

/// Inputs collected from BM-L* report JSONs.
#[derive(Debug, Default, Clone)]
pub struct FleetProjectionInputs {
    pub hardware: String,
    pub storage: String,
    pub tikv_topology: Option<String>,
    pub scylla_topology: Option<String>,
    pub per_shard_ceiling: Option<f64>,
    pub hourly_usd: f64,
    pub surreal_instances: u8,
    pub partitions_modeled: Option<u64>,
    pub clients_modeled: Option<u64>,
    pub aggregate_ops_per_sec: Option<f64>,
}

/// Computed fleet projection toward 1B ops/s aggregate.
#[derive(Debug, Serialize)]
pub struct FleetProjection {
    pub hardware: String,
    pub storage: String,
    pub tikv_topology: Option<String>,
    pub scylla_topology: Option<String>,
    pub per_shard_ops_per_sec: Option<f64>,
    pub surreal_instances: u8,
    pub partitions_modeled: Option<u64>,
    pub clients_modeled: Option<u64>,
    pub aggregate_ops_per_sec: Option<f64>,
    pub partitions_for_1e9: Option<u64>,
    pub nodes_required: Option<u64>,
    pub fleet_aggregate_ops_per_sec: Option<f64>,
    pub cost_per_million_ops_usd: Option<f64>,
    pub target_ops_per_sec: u64,
    pub disclaimer: String,
}

const TARGET_OPS: f64 = 1_000_000_000.0;

/// Compute fleet projection from measured load-tier ceilings.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn compute(inputs: &FleetProjectionInputs) -> FleetProjection {
    let ceiling = inputs.per_shard_ceiling.filter(|r| *r > 0.0);
    let partitions = ceiling.map(|r| (TARGET_OPS / r).ceil() as u64);
    let nodes = partitions.map(|p| p.max(1));
    let aggregate = inputs.aggregate_ops_per_sec.or_else(|| {
        match (
            ceiling,
            inputs.partitions_modeled,
            inputs.clients_modeled,
        ) {
            (Some(per), Some(parts), Some(clients)) => {
                Some(per * parts as f64 * clients as f64)
            }
            _ => None,
        }
    });

    let fleet_aggregate = aggregate.or_else(|| {
        ceiling
            .zip(partitions)
            .map(|(r, p)| r * p as f64)
    });
    let cost_per_m = fleet_aggregate.and_then(|agg| {
        if agg > 0.0 && inputs.hourly_usd > 0.0 {
            let nodes_f = nodes.unwrap_or(1) as f64;
            Some((inputs.hourly_usd * nodes_f / agg) * (1_000_000.0 / 3600.0))
        } else {
            None
        }
    });

    FleetProjection {
        hardware: inputs.hardware.clone(),
        storage: inputs.storage.clone(),
        tikv_topology: inputs.tikv_topology.clone(),
        scylla_topology: inputs.scylla_topology.clone(),
        per_shard_ops_per_sec: ceiling,
        surreal_instances: inputs.surreal_instances.max(1),
        partitions_modeled: inputs.partitions_modeled,
        clients_modeled: inputs.clients_modeled,
        aggregate_ops_per_sec: aggregate,
        partitions_for_1e9: partitions,
        nodes_required: nodes,
        fleet_aggregate_ops_per_sec: fleet_aggregate,
        cost_per_million_ops_usd: cost_per_m,
        target_ops_per_sec: TARGET_OPS as u64,
        disclaimer: "Projection from measured BM-L* ceilings; not a 1B/s demonstration.".into(),
    }
}
