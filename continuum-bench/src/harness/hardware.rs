//! Capture CPU, RAM, OS, root mount, and WSL host drive metadata.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use super::dimensions::Hardware;

/// Root filesystem mount metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RootMount {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub size_gib: f64,
}

/// Host physical drive behind the WSL distro (best effort).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HostDrive {
    pub model: Option<String>,
    pub media_type: Option<String>,
    pub bus_type: Option<String>,
    pub size_gib: Option<f64>,
    pub wsl_distro_path: Option<String>,
}

/// Full hardware profile embedded in each report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareDetail {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub ram_gib: f64,
    pub os: String,
    pub root_mount: RootMount,
    pub host_drive: HostDrive,
}

/// Capture live hardware metadata for the current machine.
pub fn capture(_profile: Hardware) -> Result<HardwareDetail> {
    let cpu_model = read_cpu_model();
    let cpu_cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_memory();
    let ram_gib = crate::util::u64_to_f64(sys.total_memory()) / (1024.0 * 1024.0 * 1024.0);

    let os = read_os_string();
    let root_mount = capture_root_mount()?;
    let host_drive = capture_host_drive();

    Ok(HardwareDetail {
        cpu_model,
        cpu_cores,
        ram_gib,
        os,
        root_mount,
        host_drive,
    })
}

fn read_cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split_once(':').map_or(l, |(_, v)| v.trim()))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".into())
}

fn read_os_string() -> String {
    let uname = Command::new("uname")
        .args(["-s", "-r"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    if Path::new("/proc/version").exists() {
        let version = std::fs::read_to_string("/proc/version").unwrap_or_default();
        if version.contains("Microsoft") || version.contains("WSL") {
            return format!("linux WSL2 ({uname})");
        }
    }
    format!("linux ({uname})")
}

fn capture_root_mount() -> Result<RootMount> {
    let output = Command::new("findmnt")
        .args(["-no", "SOURCE,FSTYPE,SIZE", "/"])
        .output()
        .context("findmnt failed")?;
    let line = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = line.split_whitespace().collect();

    let device = parts.first().copied().unwrap_or("/").to_string();
    let fs_type = parts.get(1).copied().unwrap_or("unknown").to_string();
    let size_str = parts.get(2).copied().unwrap_or("0");

    let size_gib = parse_size_to_gib(size_str);

    Ok(RootMount {
        device,
        mount_point: "/".into(),
        fs_type,
        size_gib,
    })
}

fn parse_size_to_gib(size: &str) -> f64 {
    let size = size.trim();
    if size.ends_with('T') {
        size.trim_end_matches('T')
            .parse::<f64>()
            .map_or(0.0, |v| v * 1024.0)
    } else if size.ends_with('G') {
        size.trim_end_matches('G').parse::<f64>().unwrap_or(0.0)
    } else if size.ends_with('M') {
        size.trim_end_matches('M')
            .parse::<f64>()
            .map_or(0.0, |v| v / 1024.0)
    } else {
        0.0
    }
}

fn capture_host_drive() -> HostDrive {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$distro = Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss' |
  Get-ItemProperty |
  Where-Object { $_.DistributionName -match 'Ubuntu|WSL' -or $_.BasePath } |
  Select-Object -First 1
if (-not $distro -or -not $distro.BasePath) { exit 1 }
$basePath = $distro.BasePath
if ($basePath -match '([A-Za-z]):') {
  $driveLetter = $matches[1]
  $part = Get-Partition -DriveLetter $driveLetter -ErrorAction SilentlyContinue
  if ($part) {
    $disk = Get-Disk -Number $part.DiskNumber -ErrorAction SilentlyContinue
    $phys = $disk | Get-PhysicalDisk -ErrorAction SilentlyContinue
    if ($phys) {
      $model = $phys.FriendlyName
      $media = $phys.MediaType
      $bus = $phys.BusType
      $size = [math]::Round($phys.Size / 1GB, 1)
      Write-Output "$basePath|$model|$media|$bus|$size"
      exit 0
    }
  }
}
Write-Output "$basePath|||"
"#;

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .output();

    let Ok(output) = output else {
        return HostDrive::default();
    };
    if !output.status.success() {
        return HostDrive::default();
    }

    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = line.split('|').collect();
    if parts.is_empty() {
        return HostDrive::default();
    }

    HostDrive {
        wsl_distro_path: Some(parts[0].to_string()),
        model: parts.get(1).filter(|s| !s.is_empty()).map(ToString::to_string),
        media_type: parts.get(2).filter(|s| !s.is_empty()).map(ToString::to_string),
        bus_type: parts.get(3).filter(|s| !s.is_empty()).map(ToString::to_string),
        size_gib: parts
            .get(4)
            .and_then(|s| s.parse::<f64>().ok()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_hardware_smoke() {
        let detail = capture(Hardware::DevWsl).expect("capture");
        assert!(!detail.cpu_model.is_empty());
        assert!(detail.cpu_cores >= 1);
        assert!(detail.ram_gib > 0.0);
        assert!(!detail.root_mount.device.is_empty());
    }

    #[test]
    fn parse_size_to_gib_works() {
        assert!((parse_size_to_gib("1.9T") - 1945.6).abs() < 1.0);
        assert!((parse_size_to_gib("19G") - 19.0).abs() < 0.1);
    }
}
