use crate::models::{GpuInfo, HardwareReport};
use serde_json::Value;
use std::{path::Path, process::Command};
use sysinfo::{Disks, System};

pub fn detect(model_directory: &Path) -> HardwareReport {
    let mut system = System::new_all();
    system.refresh_all();
    let cpu_name = system
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "未知处理器".into());
    let total_memory_bytes = system.total_memory();
    let available_memory_bytes = system.available_memory();
    let mut gpus = detect_gpus();
    enrich_nvidia_memory(&mut gpus);
    let acceleration = if gpus.is_empty() { "cpu" } else { "vulkan" }.to_string();
    let free_disk_bytes = free_space(model_directory);
    HardwareReport {
        os: format!("Windows {}", System::os_version().unwrap_or_default()),
        arch: std::env::consts::ARCH.into(),
        cpu_name,
        physical_cores: system.physical_core_count().unwrap_or(system.cpus().len()),
        logical_cores: system.cpus().len(),
        instruction_sets: instruction_sets(),
        total_memory_bytes,
        available_memory_bytes,
        gpus,
        free_disk_bytes,
        model_directory: model_directory.to_string_lossy().to_string(),
        acceleration,
    }
}

#[cfg(target_os = "windows")]
fn enrich_nvidia_memory(gpus: &mut [GpuInfo]) {
    let output = Command::new("nvidia-smi.exe")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let values = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .map(|mib| mib.saturating_mul(1024 * 1024))
        .collect::<Vec<_>>();
    for (gpu, memory) in gpus
        .iter_mut()
        .filter(|gpu| gpu.vendor == "NVIDIA")
        .zip(values)
    {
        gpu.dedicated_memory_bytes = memory;
    }
}

#[cfg(not(target_os = "windows"))]
fn enrich_nvidia_memory(_gpus: &mut [GpuInfo]) {}

fn free_space(path: &Path) -> u64 {
    if let Ok(value) = fs2::available_space(path) {
        return value;
    }
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .find(|d| path.starts_with(d.mount_point()))
        .map(|d| d.available_space())
        .unwrap_or(0)
}

fn instruction_sets() -> Vec<String> {
    let mut sets = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx") {
            sets.push("AVX".into());
        }
        if std::is_x86_feature_detected!("avx2") {
            sets.push("AVX2".into());
        }
        if std::is_x86_feature_detected!("fma") {
            sets.push("FMA".into());
        }
        if std::is_x86_feature_detected!("avx512f") {
            sets.push("AVX512".into());
        }
    }
    sets
}

#[cfg(target_os = "windows")]
fn detect_gpus() -> Vec<GpuInfo> {
    let script = "Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM,DriverVersion | ConvertTo-Json -Compress";
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
        return Vec::new();
    };
    let values = match value {
        Value::Array(items) => items,
        item => vec![item],
    };
    values
        .into_iter()
        .filter_map(|item| {
            let name = item.get("Name")?.as_str()?.to_string();
            if name.contains("Microsoft Basic") || name.contains("Remote Display") {
                return None;
            }
            let lower = name.to_ascii_lowercase();
            let vendor = if lower.contains("nvidia") {
                "NVIDIA"
            } else if lower.contains("amd") || lower.contains("radeon") {
                "AMD"
            } else if lower.contains("intel") {
                "Intel"
            } else {
                "Other"
            };
            let memory = item.get("AdapterRAM").and_then(value_to_u64).unwrap_or(0);
            Some(GpuInfo {
                name,
                vendor: vendor.into(),
                dedicated_memory_bytes: memory,
                driver_version: item
                    .get("DriverVersion")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

fn value_to_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}
