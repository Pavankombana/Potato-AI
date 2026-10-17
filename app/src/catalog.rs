use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    pub category: String,
    pub filename: String,
    pub download_url: String,
    pub file_size_mb: u64,
    pub ram_needed_mb: u64,
    pub speed_rating: String,
    pub best_for: String,
}

pub fn get_curated_models() -> Vec<ModelSpec> {
    vec![
        ModelSpec {
            id: "smollm2-135m".to_string(),
            name: "SmolLM2 135M".to_string(),
            category: "Ultra-Lightweight".to_string(),
            filename: "SmolLM2-135M-Instruct-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/SmolLM2-135M-Instruct-GGUF/resolve/main/SmolLM2-135M-Instruct-Q4_K_M.gguf".to_string(),
            file_size_mb: 101,
            ram_needed_mb: 250,
            speed_rating: "55 - 85 tok/s".to_string(),
            best_for: "Ultra-lightweight assistant for low-end CPUs, instant responses.".to_string(),
        },
        ModelSpec {
            id: "smollm2-360m".to_string(),
            name: "SmolLM2 360M".to_string(),
            category: "Lightweight Chat".to_string(),
            filename: "SmolLM2-360M-Instruct-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF/resolve/main/SmolLM2-360M-Instruct-Q4_K_M.gguf".to_string(),
            file_size_mb: 258,
            ram_needed_mb: 400,
            speed_rating: "45 - 65 tok/s".to_string(),
            best_for: "Low-RAM systems, instant answers, minimal resource usage.".to_string(),
        },
        ModelSpec {
            id: "qwen2.5-0.5b".to_string(),
            name: "Qwen 2.5 0.5B".to_string(),
            category: "Logic & Math".to_string(),
            filename: "qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            file_size_mb: 469,
            ram_needed_mb: 750,
            speed_rating: "35 - 50 tok/s".to_string(),
            best_for: "Step-by-step logic, math problems, concise explanations.".to_string(),
        },
        ModelSpec {
            id: "qwen2.5-coder-0.5b".to_string(),
            name: "Qwen 2.5 Coder 0.5B".to_string(),
            category: "Coding & Dev".to_string(),
            filename: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf".to_string(),
            file_size_mb: 469,
            ram_needed_mb: 750,
            speed_rating: "35 - 50 tok/s".to_string(),
            best_for: "Fast code generation and debugging on low-end CPUs.".to_string(),
        },
        ModelSpec {
            id: "tinyllama-1.1b".to_string(),
            name: "TinyLlama 1.1B".to_string(),
            category: "Fast Chat".to_string(),
            filename: "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".to_string(),
            file_size_mb: 638,
            ram_needed_mb: 950,
            speed_rating: "28 - 42 tok/s".to_string(),
            best_for: "Compact 1.1B chat model, smooth inference on older CPUs.".to_string(),
        },
        ModelSpec {
            id: "llama-3.2-1b".to_string(),
            name: "Llama 3.2 1B".to_string(),
            category: "General Knowledge".to_string(),
            filename: "Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_string(),
            file_size_mb: 770,
            ram_needed_mb: 1100,
            speed_rating: "22 - 32 tok/s".to_string(),
            best_for: "Meta general language model. Writing, research, conversation.".to_string(),
        },
        ModelSpec {
            id: "qwen2.5-coder-1.5b".to_string(),
            name: "Qwen 2.5 Coder 1.5B".to_string(),
            category: "Advanced Coding".to_string(),
            filename: "qwen2.5-coder-1.5b-instruct-q4_k_m.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf".to_string(),
            file_size_mb: 1066,
            ram_needed_mb: 1550,
            speed_rating: "18 - 25 tok/s".to_string(),
            best_for: "High quality code logic, multi-function scripts, complex programming.".to_string(),
        },
        ModelSpec {
            id: "qwen2.5-1.5b".to_string(),
            name: "Qwen 2.5 1.5B".to_string(),
            category: "General & Reasoning".to_string(),
            filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            file_size_mb: 1066,
            ram_needed_mb: 1550,
            speed_rating: "18 - 25 tok/s".to_string(),
            best_for: "Well-rounded general intelligence, multi-language dialogue, structured reasoning.".to_string(),
        },
        ModelSpec {
            id: "smollm2-1.7b".to_string(),
            name: "SmolLM2 1.7B".to_string(),
            category: "General & Knowledge".to_string(),
            filename: "SmolLM2-1.7B-Instruct-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/SmolLM2-1.7B-Instruct-GGUF/resolve/main/SmolLM2-1.7B-Instruct-Q4_K_M.gguf".to_string(),
            file_size_mb: 1007,
            ram_needed_mb: 1500,
            speed_rating: "18 - 25 tok/s".to_string(),
            best_for: "Balanced performance for general questions, summaries, and writing.".to_string(),
        },
        ModelSpec {
            id: "deepseek-r1-1.5b".to_string(),
            name: "DeepSeek R1 Distill 1.5B".to_string(),
            category: "Deep Reasoning".to_string(),
            filename: "DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf".to_string(),
            file_size_mb: 1066,
            ram_needed_mb: 1550,
            speed_rating: "18 - 26 tok/s".to_string(),
            best_for: "Step-by-step reasoning, logic puzzles, deep problem solving.".to_string(),
        },
        ModelSpec {
            id: "gemma-2-2b".to_string(),
            name: "Gemma 2 2B".to_string(),
            category: "High Accuracy".to_string(),
            filename: "gemma-2-2b-it-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf".to_string(),
            file_size_mb: 1629,
            ram_needed_mb: 2300,
            speed_rating: "12 - 18 tok/s".to_string(),
            best_for: "Google DeepMind compact model with high factual accuracy.".to_string(),
        },
        ModelSpec {
            id: "llama-3.2-3b".to_string(),
            name: "Llama 3.2 3B".to_string(),
            category: "High Capability".to_string(),
            filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            file_size_mb: 1926,
            ram_needed_mb: 2800,
            speed_rating: "10 - 16 tok/s".to_string(),
            best_for: "Meta flagship compact model for nuanced instructions, creative writing, and deep context.".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curated_models_validity() {
        let models = get_curated_models();
        assert!(models.len() >= 12, "Should have expanded catalog");
        let mut ids = std::collections::HashSet::new();
        let mut filenames = std::collections::HashSet::new();
        let mut urls = std::collections::HashSet::new();
        for m in &models {
            assert!(!m.id.is_empty());
            assert!(!m.name.is_empty());
            assert!(!m.category.is_empty());
            assert!(m.filename.ends_with(".gguf"));
            assert!(m.download_url.starts_with("https://huggingface.co/"));
            assert!(m.file_size_mb > 0);
            assert!(m.ram_needed_mb > 0);
            assert!(!m.speed_rating.is_empty());
            assert!(!m.best_for.is_empty());

            assert!(ids.insert(&m.id), "Duplicate ID: {}", m.id);
            assert!(filenames.insert(&m.filename), "Duplicate filename: {}", m.filename);
            assert!(urls.insert(&m.download_url), "Duplicate URL: {}", m.download_url);
        }
    }
}
