//! Combines system info: sysinfo 0.38 with iced GPU iced
//! sysinfo doesn't query GPU

use std::time::Duration;

use crate::helpers::humanize_duration;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    //GPU - from iced::system::Information
    pub graphics_adapter: String,
    pub graphics_backend: String,

    //Everything else from sysinfo 0.38
    pub host_name: Option<String>,
    pub system_name: Option<String>,
    pub system_version: Option<String>,
    pub kernel_version: Option<String>,

    pub cpu_brand: String,
    pub cpu_cores_physical: Option<usize>,
    pub cpu_cores_logical: usize,
    pub cpu_usage: f32,

    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_available: u64,

    pub uptime: Duration,
    pub boot_time_unix: u64,
}

/// Synchronous sysinfo collection - via `tokio::task::spawn_blocking`
#[derive(Debug, Clone, Default)]
pub struct SysinfoData {
    pub host_name: Option<String>,
    pub system_name: Option<String>,
    pub system_version: Option<String>,
    pub kernel_version: Option<String>,
    pub cpu_brand: String,
    pub cpu_cores_physical: Option<usize>,
    pub cpu_cores_logical: usize,
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_available: u64,
    pub uptime: Duration,
    pub boot_time_unix: u64,
}

impl SysinfoData {
    pub fn collect() -> Self {
        use sysinfo::System;

        let mut system = System::new_all();

        // Two refreshes for accurate CPU% — first establishes baseline,
        // second computes the delta. Single refresh returns 0% always.
        system.refresh_all();

        std::thread::sleep(std::time::Duration::from_millis(200));
        system.refresh_cpu_all();

        let cpu_brand = system
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        let cpu_usage = system.global_cpu_usage();

        Self {
            host_name: System::host_name(),
            system_name: System::name(),
            system_version: System::os_version(),
            kernel_version: System::kernel_version(),
            cpu_brand,
            cpu_cores_physical: System::physical_core_count(),
            cpu_cores_logical: system.cpus().len(),
            cpu_usage,
            memory_total: system.total_memory(),
            memory_used: system.used_memory(),
            memory_available: system.available_memory(),
            uptime: Duration::from_secs(System::uptime()),
            boot_time_unix: System::boot_time(),
        }
    }
}

impl SystemInfo {
    // Combines the two parallel fetches into a single structure
    pub fn from_parts(iced: &iced::system::Information, extras: &SysinfoData) -> Self {
        Self {
            graphics_adapter: iced.graphics_adapter.clone(),
            graphics_backend: iced.graphics_backend.clone(),

            host_name: extras.host_name.clone(),
            system_name: extras.system_name.clone(),
            system_version: extras.system_version.clone(),
            kernel_version: extras.kernel_version.clone(),

            cpu_brand: extras.cpu_brand.clone(),
            cpu_cores_physical: extras.cpu_cores_physical,
            cpu_cores_logical: extras.cpu_cores_logical,
            cpu_usage: extras.cpu_usage,

            memory_available: extras.memory_available,
            memory_used: extras.memory_used,
            memory_total: extras.memory_total,

            uptime: extras.uptime,
            boot_time_unix: extras.boot_time_unix,
        }
    }

    pub fn to_clipboard_string(&self) -> String {
        format!(
            "Snapdash {snap_version}
Hostname: {host}
OS: {os} {ver}
Kernel: {kernel}
CPU: {cpu} ({cores_phys}/{cores_log} cores, {load:.1}% load)
Memory: {mem_used:.1} / {mem_total:.1} GB ({mem_avail:.1} GB available)
GPU: {gpu} ({backend})
Uptime: {uptime}",
            snap_version = env!("CARGO_PKG_VERSION"),
            host = self.host_name.as_deref().unwrap_or("?"),
            os = self.system_name.as_deref().unwrap_or("?"),
            ver = self.system_version.as_deref().unwrap_or(""),
            kernel = self.kernel_version.as_deref().unwrap_or("?"),
            cpu = self.cpu_brand,
            cores_phys = self.cpu_cores_physical.unwrap_or(0),
            cores_log = self.cpu_cores_logical,
            load = self.cpu_usage,
            mem_used = self.memory_used as f64 / 1_073_741_824.0,
            mem_total = self.memory_total as f64 / 1_073_741_824.0,
            mem_avail = self.memory_available as f64 / 1_073_741_824.0,
            gpu = self.graphics_adapter,
            backend = self.graphics_backend,
            uptime = humanize_duration(self.uptime),
        )
    }

    pub fn to_md_string(&self) -> String {
        format!(
            "|Field | Value |
| ----- | ----- |
| Snapdash | `{snap_version}` |
| Hostname |  `{host}` |
| OS | `{os} {ver}` |
| Kernel | `{kernel}` |
| CPU | `{cpu}` ({cores_phys}/{cores_log} cores, {load:.1}% load) |
| Memory | {mem_used:.1} / {mem_total:.1} GB |
| GPU | `{gpu}` ({backend}) |
| Uptime | {uptime} |",
            snap_version = env!("CARGO_PKG_VERSION"),
            host = self.host_name.as_deref().unwrap_or("?"),
            os = self.system_name.as_deref().unwrap_or("?"),
            ver = self.system_version.as_deref().unwrap_or(""),
            kernel = self.kernel_version.as_deref().unwrap_or("?"),
            cpu = self.cpu_brand,
            cores_phys = self.cpu_cores_physical.unwrap_or(0),
            cores_log = self.cpu_cores_logical,
            load = self.cpu_usage,
            mem_used = self.memory_used as f64 / 1_073_741_824.0,
            mem_total = self.memory_total as f64 / 1_073_741_824.0,
            gpu = self.graphics_adapter,
            backend = self.graphics_backend,
            uptime = humanize_duration(self.uptime),
        )
    }
}
