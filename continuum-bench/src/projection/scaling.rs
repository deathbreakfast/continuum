//! Multi-node scaling curve from peak BM-M4 reports per topology.

use std::path::Path;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::harness::{ScyllaTopology, TikvTopology};

const TARGET_OPS: f64 = 1_000_000_000.0;

/// One point on the storage-node scaling curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPoint {
    pub topology: String,
    pub storage_nodes: u8,
    pub peak_ops_per_sec: f64,
    pub peak_client_count: u64,
    pub peak_p99_ms: Option<f64>,
    pub vs_baseline: Option<f64>,
    pub ops_per_storage_node: f64,
    pub report_file: String,
}

/// Full scaling curve for one storage backend + hardware profile.
#[derive(Debug, Serialize)]
pub struct ScalingCurve {
    pub hardware: String,
    pub storage: String,
    pub baseline_topology: String,
    pub baseline_peak_ops_per_sec: Option<f64>,
    pub points: Vec<ScalingPoint>,
    pub clusters_for_1e9: Option<u64>,
    pub storage_nodes_for_1e9: Option<u64>,
    pub disclaimer: String,
}

fn storage_nodes_for_topology(storage: &str, topo: &str) -> Option<u8> {
    if storage == "scylla" {
        ScyllaTopology::parse(topo).map(ScyllaTopology::storage_node_count)
    } else if storage == "tikv-raw" {
        TikvTopology::parse(topo).map(TikvTopology::storage_node_count)
    } else {
        None
    }
}

fn topo_from_report(v: &Value, storage: &str, report_file: &str) -> Option<String> {
    let dims = v.get("dimensions")?;
    if storage == "scylla" {
        if let Some(t) = dims.get("scylla_topology").and_then(|t| t.as_str()) {
            return Some(t.to_string());
        }
        for slug in ["scylla-4n", "scylla-3n", "scylla-2n", "scylla-1"] {
            if report_file.contains(slug) {
                return Some(slug.into());
            }
        }
        Some("scylla-1".into())
    } else {
        if let Some(t) = dims.get("tikv_topology").and_then(|t| t.as_str()) {
            return Some(t.to_string());
        }
        for slug in [
            "tikv-scale-5",
            "tikv-scale-4",
            "tikv-ha-3",
            "tikv-ha-2",
            "tikv-minimal",
        ] {
            if report_file.contains(slug) {
                return Some(slug.into());
            }
        }
        Some("tikv-minimal".into())
    }
}

fn matches_hardware_storage(v: &Value, hardware: &str, storage: &str) -> bool {
    let Some(d) = v.get("dimensions").and_then(|x| x.as_object()) else {
        return false;
    };
    d.get("hardware").and_then(|h| h.as_str()) == Some(hardware)
        && d.get("storage").and_then(|s| s.as_str()) == Some(storage)
}

/// Load peak BM-M4 achieved rate per topology slug.
pub fn load_scaling_curve(reports_dir: &Path, hardware: &str, storage: &str) -> Result<ScalingCurve> {
    let baseline_topology = if storage == "scylla" {
        "scylla-1".into()
    } else if storage == "tikv-raw" {
        "tikv-minimal".into()
    } else {
        bail!("scaling curve supports scylla or tikv-raw only");
    };

    let mut best: std::collections::HashMap<String, (f64, u64, Option<f64>, String)> =
        std::collections::HashMap::new();

    for entry in std::fs::read_dir(reports_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let v: Value = serde_json::from_str(&text)?;
        if v.get("experiment_id").and_then(|e| e.as_str()) != Some("bm-m4") {
            continue;
        }
        if !matches_hardware_storage(&v, hardware, storage) {
            continue;
        }
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let Some(topo) = topo_from_report(&v, storage, &fname) else {
            continue;
        };
        let rate = v
            .pointer("/metrics/achieved_ops_per_sec")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        let ck = v
            .pointer("/metrics/client_count")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let p99 = v.pointer("/metrics/p99_ms").and_then(|x| x.as_f64());
        best
            .entry(topo)
            .and_modify(|(best_rate, best_ck, best_p99, best_file)| {
                if rate > *best_rate {
                    *best_rate = rate;
                    *best_ck = ck;
                    *best_p99 = p99;
                    *best_file = fname.clone();
                }
            })
            .or_insert((rate, ck, p99, fname));
    }

    if best.is_empty() {
        bail!(
            "no BM-M4 reports for {hardware}/{storage} in {}",
            reports_dir.display()
        );
    }

    let baseline_peak = best.get(&baseline_topology).map(|(r, _, _, _)| *r);

    let mut points: Vec<ScalingPoint> = best
        .into_iter()
        .filter_map(|(topo, (peak_ops, peak_ck, p99, file))| {
            let nodes = storage_nodes_for_topology(storage, &topo)?;
            if nodes == 0 {
                return None;
            }
            let vs = baseline_peak.filter(|b| *b > 0.0).map(|b| peak_ops / b);
            Some(ScalingPoint {
                topology: topo,
                storage_nodes: nodes,
                peak_ops_per_sec: peak_ops,
                peak_client_count: peak_ck,
                peak_p99_ms: p99,
                vs_baseline: vs,
                ops_per_storage_node: peak_ops / f64::from(nodes),
                report_file: file,
            })
        })
        .collect();
    points.sort_by_key(|p| p.storage_nodes);

    let best_cluster = points.iter().map(|p| p.peak_ops_per_sec).fold(0.0_f64, f64::max);
    let clusters_for_1e9 = if best_cluster > 0.0 {
        Some((TARGET_OPS / best_cluster).ceil() as u64)
    } else {
        None
    };
    let storage_nodes_for_1e9 = clusters_for_1e9.and_then(|clusters| {
        points
            .iter()
            .max_by(|a, b| {
                a.peak_ops_per_sec
                    .partial_cmp(&b.peak_ops_per_sec)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| clusters * u64::from(p.storage_nodes))
    });

    Ok(ScalingCurve {
        hardware: hardware.into(),
        storage: storage.into(),
        baseline_topology,
        baseline_peak_ops_per_sec: baseline_peak,
        points,
        clusters_for_1e9,
        storage_nodes_for_1e9,
        disclaimer: "Scaling curve from peak BM-M4 per topology; N=1 baseline may be colocated.".into(),
    })
}

pub fn render_scaling_markdown(curve: &ScalingCurve) -> String {
    let mut lines = vec![
        "# Continuum storage scaling curve".into(),
        String::new(),
        format!("- hardware: `{}`", curve.hardware),
        format!("- storage: `{}`", curve.storage),
        format!("- baseline topology: `{}`", curve.baseline_topology),
    ];
    if let Some(b) = curve.baseline_peak_ops_per_sec {
        lines.push(format!("- baseline peak: {b:.0} ops/s"));
    }
    lines.push(String::new());
    lines.push("| nodes | topology | peak ops/s | C@peak | vs N=1 | ops/s/node |".into());
    lines.push("| --- | --- | --- | --- | --- | --- |".into());
    for p in &curve.points {
        let vs = p
            .vs_baseline
            .map(|v| format!("{v:.2}×"))
            .unwrap_or_else(|| "—".into());
        lines.push(format!(
            "| {} | {} | {:.0} | {} | {} | {:.0} |",
            p.storage_nodes, p.topology, p.peak_ops_per_sec, p.peak_client_count, vs, p.ops_per_storage_node
        ));
    }
    if let Some(c) = curve.clusters_for_1e9 {
        lines.push(String::new());
        lines.push(format!("- clusters for 1B/s (best peak): {c}"));
    }
    if let Some(n) = curve.storage_nodes_for_1e9 {
        lines.push(format!("- storage nodes for 1B/s (best peak): {n}"));
    }
    lines.push(String::new());
    lines.push(curve.disclaimer.clone());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tempfile::TempDir;

    #[test]
    fn scaling_curve_picks_peak_per_topology() {
        let dir = TempDir::new().unwrap();
        let report = r#"{
            "experiment_id": "bm-m4",
            "dimensions": {"hardware": "aws-t3-medium", "storage": "scylla", "scylla_topology": "scylla-2n"},
            "metrics": {"achieved_ops_per_sec": 5000.0, "client_count": 256, "p99_ms": 40.0}
        }"#;
        let mut f = std::fs::File::create(dir.path().join("a.json")).unwrap();
        write!(f, "{report}").unwrap();
        let mut f2 = std::fs::File::create(dir.path().join("b.json")).unwrap();
        write!(
            f2,
            r#"{{
            "experiment_id": "bm-m4",
            "dimensions": {{"hardware": "aws-t3-medium", "storage": "scylla", "scylla_topology": "scylla-1"}},
            "metrics": {{"achieved_ops_per_sec": 3318.0, "client_count": 256, "p99_ms": 70.0}}
        }}"#
        )
        .unwrap();
        let curve = load_scaling_curve(dir.path(), "aws-t3-medium", "scylla").unwrap();
        assert_eq!(curve.points.len(), 2);
        assert!(curve.clusters_for_1e9.unwrap() > 0);
    }
}
