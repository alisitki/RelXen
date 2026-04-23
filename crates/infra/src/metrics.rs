use std::sync::Mutex;

use sysinfo::System;

use relxen_app::MetricsPort;
use relxen_domain::SystemMetrics;

pub struct SystemMetricsCollector {
    system: Mutex<System>,
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self {
            system: Mutex::new(System::new_all()),
        }
    }
}

impl MetricsPort for SystemMetricsCollector {
    fn snapshot(&self) -> SystemMetrics {
        let mut system = self.system.lock().expect("system mutex poisoned");
        system.refresh_cpu_usage();
        system.refresh_memory();

        SystemMetrics {
            cpu_usage_percent: system.global_cpu_usage(),
            memory_used_bytes: system.used_memory(),
            memory_total_bytes: system.total_memory(),
            task_count: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            collected_at: relxen_app::now_ms(),
        }
    }
}
