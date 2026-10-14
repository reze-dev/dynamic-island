use isle_core::SystemStats;
use sysinfo::{LoadAvg, MemoryRefreshKind, RefreshKind, System};

pub struct Sampler {
    system: System,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(Default::default())
                    .with_memory(MemoryRefreshKind::nothing().with_ram()),
            ),
        }
    }

    pub fn sample(&mut self) -> SystemStats {
        self.system.refresh_cpu_usage();
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());

        let cpus = self.system.cpus();
        let cpu_percent = if cpus.is_empty() {
            0.0
        } else {
            (cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32 * 100.0).round()
                / 100.0
        };

        SystemStats {
            cpu_percent,
            memory_used_bytes: self.system.used_memory(),
            memory_total_bytes: self.system.total_memory(),
            uptime_secs: System::uptime(),
            load_average: load_average(),
        }
    }
}

fn load_average() -> Option<f32> {
    let LoadAvg {
        one,
        five,
        fifteen: _,
    } = System::load_average();
    let representative = if one > 0.0 { one } else { five };
    (representative > 0.0).then(|| ((representative * 100.0).round() / 100.0) as f32)
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampling_produces_a_sane_report() {
        let mut sampler = Sampler::new();
        sampler.sample();
        let stats = sampler.sample();

        assert!(
            stats.cpu_percent.is_finite() && stats.cpu_percent >= 0.0 && stats.cpu_percent <= 100.0,
            "cpu usage should be a finite percentage, got {}",
            stats.cpu_percent
        );
        assert!(
            stats.memory_total_bytes > 0,
            "total memory should be reported"
        );
        assert!(
            stats.memory_used_bytes <= stats.memory_total_bytes,
            "used memory cannot exceed total"
        );
        assert!(stats.uptime_secs > 0, "uptime should be positive");
    }

    #[test]
    fn consecutive_samples_are_monotonic_in_uptime() {
        let mut sampler = Sampler::new();
        sampler.sample();
        let first = sampler.sample().uptime_secs;
        let second = sampler.sample();
        assert!(second.uptime_secs >= first);
    }

    #[test]
    fn stats_survive_a_json_round_trip() {
        let stats = Sampler::new().sample();
        let json = serde_json::to_string(&stats).unwrap();
        let back: SystemStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.memory_used_bytes, back.memory_used_bytes);
        assert_eq!(stats.cpu_percent, back.cpu_percent);
    }
}
