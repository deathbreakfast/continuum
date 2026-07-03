//! Markdown rendering for fleet projection output.

use super::model::FleetProjection;

/// Render a human-readable projection summary for stdout.
pub fn render_markdown(p: &FleetProjection) -> String {
    let mut lines = vec![
        "# Continuum fleet projection".into(),
        String::new(),
        format!("- hardware: `{}`", p.hardware),
        format!("- storage: `{}`", p.storage),
    ];
    if let Some(t) = &p.tikv_topology {
        lines.push(format!("- tikv_topology: `{t}`"));
    }
    lines.push(format!("- target: {} ops/s", p.target_ops_per_sec));
    if let Some(r) = p.per_shard_ops_per_sec {
        lines.push(format!("- per-shard ceiling: {r:.0} ops/s"));
    }
    if let Some(n) = p.partitions_modeled {
        lines.push(format!("- partitions modeled (Track P): {n}"));
    }
    if let Some(n) = p.clients_modeled {
        lines.push(format!("- clients modeled (Track M): {n}"));
    }
    if let Some(r) = p.aggregate_ops_per_sec {
        lines.push(format!("- measured aggregate: {r:.0} ops/s"));
    }
    if let Some(n) = p.partitions_for_1e9 {
        lines.push(format!("- partitions for 1B/s: {n}"));
    }
    if let Some(n) = p.nodes_required {
        lines.push(format!("- nodes required (1 partition/node): {n}"));
    }
    if let Some(c) = p.cost_per_million_ops_usd {
        lines.push(format!("- cost estimate: ${c:.4} / M ops (compute only)"));
    }
    lines.push(String::new());
    lines.push(p.disclaimer.clone());
    lines.join("\n")
}
