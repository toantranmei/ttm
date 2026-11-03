//! How many agents this machine runs at once: `clamp(cpus / 4, 1, 3)`, lowered so each agent has 4 GB of free RAM.

/// RAM each agent should have (bytes).
pub const RAM_PER_AGENT: u64 = 4 * 1024 * 1024 * 1024;

pub fn default_max_parallel(cpus: usize, available_ram: u64) -> usize {
    let by_cpu = (cpus / 4).clamp(1, 3);
    let by_ram = usize::try_from(available_ram / RAM_PER_AGENT)
        .unwrap_or(usize::MAX)
        .max(1);
    by_cpu.min(by_ram)
}

/// The default for this machine.
pub fn detect() -> usize {
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let ram = sysinfo::System::new()
        .map(|mut s| {
            s.refresh_memory();
            s.available_memory()
        })
        .unwrap_or(RAM_PER_AGENT);
    default_max_parallel(cpus, ram)
}

/// Effective capacity: the configured value (or the detected default), never below 1.
pub fn effective(configured: Option<usize>) -> usize {
    configured.unwrap_or_else(detect).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn cpu_and_ram_bounds() {
        assert_eq!(default_max_parallel(2, 64 * GB), 1);
        assert_eq!(default_max_parallel(8, 64 * GB), 2);
        assert_eq!(default_max_parallel(32, 64 * GB), 3);
        assert_eq!(default_max_parallel(32, 9 * GB), 2);
        assert_eq!(default_max_parallel(32, GB), 1);
        assert!(effective(Some(0)) >= 1);
        assert_eq!(effective(Some(5)), 5);
    }
}
