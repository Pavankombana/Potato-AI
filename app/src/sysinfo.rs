use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSpecs {
    pub cpu_name: String,
    pub logical_cores: usize,
    pub total_ram_gb: f32,
    pub available_ram_gb: f32,
    pub tier: String,
    pub tier_description: String,
    pub can_run_summary: String,
    pub recommended_model: String,
    pub recommended_model_name: String,
    pub recommended_threads: usize,
    pub optimal_batch_size: usize,
    pub optimization_tip: String,
}

pub fn detect_cpu_name() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(output) = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "/v",
                "ProcessorNameString",
            ])
            .creation_flags(0x08000000)
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("ProcessorNameString") {
                    let parts: Vec<&str> = line.split("REG_SZ").collect();
                    if parts.len() > 1 {
                        let name = parts[1].trim();
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }
    "Generic x86_64 Processor".to_string()
}

pub fn detect_ram() -> (f32, f32) {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(stat: *mut MemoryStatusEx) -> i32;
        }

        let mut stat: MemoryStatusEx = unsafe { std::mem::zeroed() };
        stat.dw_length = std::mem::size_of::<MemoryStatusEx>() as u32;
        if unsafe { GlobalMemoryStatusEx(&mut stat) } != 0 {
            let total = (stat.ull_total_phys as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let avail = (stat.ull_avail_phys as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            return (total, avail);
        }
    }
    (4.0, 2.0)
}

pub fn get_system_specs() -> SystemSpecs {
    let cpu_name = detect_cpu_name();
    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let (total_ram_gb, available_ram_gb) = detect_ram();

    // Determine optimal threads keeping headroom for OS and browser UI
    let recommended_threads = if logical_cores <= 2 {
        1
    } else if logical_cores <= 4 {
        3
    } else if logical_cores <= 6 {
        4
    } else {
        (logical_cores - 2).min(6)
    };

    // System categorization based on physical RAM
    let (tier, tier_description, can_run_summary, recommended_model, recommended_model_name, tip) =
        if total_ram_gb <= 5.5 {
            (
                "Entry / Low-Spec (4 GB RAM)".to_string(),
                "Optimized for 0.5B models (Qwen 0.5B, SmolLM2 360M). Zero lag and low RAM consumption.".to_string(),
                "0.5B Models (Qwen Coder 0.5B, SmolLM2 360M)".to_string(),
                "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf".to_string(),
                "Qwen 2.5 Coder 0.5B".to_string(),
                "On 4 GB RAM systems, 0.5B models run at peak speed (35-65 tok/s) without memory pressure.".to_string(),
            )
        } else if total_ram_gb <= 11.0 {
            (
                "Balanced / Mid-Spec (8 GB RAM)".to_string(),
                "Smoothly runs 0.5B, 1B, and 1.5B models with high coding and reasoning precision.".to_string(),
                "0.5B, 1.0B, & 1.5B Models (Qwen 1.5B, Llama 3.2 1B, DeepSeek R1 1.5B)".to_string(),
                "qwen2.5-coder-1.5b-instruct-q4_k_m.gguf".to_string(),
                "Qwen 2.5 Coder 1.5B".to_string(),
                "Your 8 GB RAM provides ample headroom for advanced 1.5B models with full 2K context.".to_string(),
            )
        } else {
            (
                "High-Performance (16+ GB RAM)".to_string(),
                "Can comfortably run any 0.5B - 2B+ model with multi-turn context and reasoning.".to_string(),
                "All Models: 0.5B, 1B, 1.5B, & 2B (Gemma 2 2B, DeepSeek R1 1.5B, Qwen 1.5B)".to_string(),
                "DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf".to_string(),
                "DeepSeek R1 Distill 1.5B".to_string(),
                "Your system has extensive RAM; you can run deep reasoning or 2B models with zero constraints.".to_string(),
            )
        };

    SystemSpecs {
        cpu_name,
        logical_cores,
        total_ram_gb: (total_ram_gb * 10.0).round() / 10.0,
        available_ram_gb: (available_ram_gb * 10.0).round() / 10.0,
        tier,
        tier_description,
        can_run_summary,
        recommended_model,
        recommended_model_name,
        recommended_threads,
        optimal_batch_size: 256,
        optimization_tip: tip,
    }
}
