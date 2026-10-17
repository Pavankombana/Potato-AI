# Potato AI

Lightweight, 100% offline, native desktop AI application designed for Windows systems. Optimized specifically for CPU-only execution without requiring any dedicated graphics card.

Runs smoothly on budget PCs and low-spec machines (Intel Core i3/i5, 4 GB RAM, zero GPU), and operates entirely self-contained from a USB pendrive.

---

## Features

- **100% Offline & Private**: All inference and calculations run locally on your CPU. No telemetry, no cloud calls, zero external network dependency during chat.
- **Embedded Branding**: The Windows executable (`PotatoAI.exe`) includes the embedded Potato AI logo icon visible across Windows Explorer and taskbar.
- **Universal Portability**: Runs cleanly across all Windows 10 and Windows 11 machines. No external runtimes required (no Rust, Python, .NET, or VC++ redistributables). Includes automatic fallback browser launching (Edge, Chrome, Brave, system default).
- **Instant Fresh Chat on Startup**: Always opens directly to a clean, fresh chat session ready for immediate user input, while full chat history remains accessible in the sidebar.
- **Tactile UI Interactions**: Micro-interactions with tactile active button press animations and immediate feedback on all controls and the Save Configuration button.
- **Rock-Solid Local Model Browser**: Win32 STA file picker with silent PowerShell Forms fallback. Handles cancellation gracefully without error toasts and automatically imports and activates selected GGUF models.
- **Comprehensive About Center**: In-app modal detailing architecture, live system hardware profile, active model specifications, keyboard shortcuts, and privacy guarantee.
- **Keyboard Shortcuts**:
  - `Enter`: Send message
  - `Shift + Enter`: Multi-line prompt
  - `Ctrl + K` / `Ctrl + N`: Start fresh chat session
  - `Ctrl + /`: Toggle About & Help modal
  - `Esc`: Dismiss modals and dropdowns
- **Zero Console Flashes**: Silent native execution. No black command prompt windows popping up on startup or shutdown.
- **Maximized Window Mode**: Opens in maximized standard window mode with native minimize, restore, and close buttons.
- **Real-Time Token Streaming**: Direct Server-Sent Events (SSE) token streaming from the local CPU engine.
- **Model Display & Speed Metrics**: Displays the active model name alongside generation speed (e.g., `Qwen 2.5 Coder 0.5B · 85.2 tok/s`).
- **Instant Stop**: Halt token generation immediately at any time.
- **Chat Management**:
  - Saved automatically to local storage.
  - Hover 3-dots menu on each conversation for exporting to Markdown (`.md`) or deleting.
  - One-click conversation import from Markdown (`.md`) and JSON (`.json`) files.
- **In-App Model Store**: One-click download with real-time percentage, downloaded MB, total MB, and speed metrics.
- **Auto-Healing Engine**: Health monitor checks inference engine status every 3 seconds and automatically restarts it if interrupted.
- **Clean Shutdown**: Zero orphan background processes or memory leaks when the application window is closed.

---

## Curated Models

Potato AI includes an in-app catalog of 8 verified, high-efficiency models optimized for CPU inference:

| Model | Size | Best For |
| :--- | :--- | :--- |
| **Qwen 2.5 Coder 0.5B** | 390 MB | Ultra-fast code generation, scripts, and terminal commands |
| **SmolLM2 360M** | 258 MB | Lightweight chat, fast summaries, and instant answers |
| **Llama 3.2 1B** | 770 MB | Meta general knowledge, reasoning, and writing |
| **DeepSeek R1 Distill 1.5B** | 1.06 GB | Step-by-step logic, reasoning, and complex problem solving |
| **SmolLM2 1.7B** | 1.00 GB | General knowledge, long context, and multi-step tasks |
| **Qwen 2.5 Coder 1.5B** | 980 MB | Advanced programming, multi-language support, refactoring |
| **Qwen 2.5 0.5B** | 390 MB | Math, rapid explanations, and concise logic |
| **Gemma 2 2B** | 1.63 GB | Google DeepMind compact model with high conversational quality |

Custom GGUF models can also be downloaded by pasting any HuggingFace direct download link or imported from local storage.

---

## Architecture

- **Application Shell**: Native Rust binary (`PotatoAI.exe`) with `windows_subsystem = "windows"` and embedded Windows icon resources.
- **UI Runtime**: Native standalone window (`--app=http://127.0.0.1:[port] --start-maximized`).
- **Backend Server**: Embedded Rust multi-threaded HTTP/SSE server.
- **Inference Engine**: Embedded `llama-server.exe` running in CPU-only mode with AVX/AVX2 acceleration.
- **Lifecycle Management**: Bidirectional heartbeat monitor and beacon shutdown ensuring zero orphan processes.

---

## Getting Started

### Prerequisites
- Windows 10 or Windows 11 (64-bit)
- Microsoft Edge (pre-installed on Windows 10/11) or Google Chrome / Brave
- 4 GB RAM minimum

### Running Potato AI
1. Clone this repository or download the release.
2. Double-click `PotatoAI.exe` (or `Run.bat`).
3. The interface will open with the opening logo animation.
4. If no model is installed, open the **Model Store** (from top-right status or model pill) and click **Download** next to any curated model (e.g. Qwen 2.5 Coder 0.5B or SmolLM2 360M).
5. Start chatting immediately.

### Portable Pendrive Execution
Copy the entire folder containing `PotatoAI.exe`, `engine/`, `potato_config.json`, and `potato_logo.svg` to any USB flash drive. Plug it into any compatible Windows PC and launch `PotatoAI.exe` directly.

---

## Building from Source

To compile Potato AI from source code:

1. Install Rust and Cargo (via `rustup.rs`).
2. Ensure MinGW or MSVC toolchain is available (with `windres` or `rc.exe` for icon resource embedding).
3. Open a terminal in the `app` directory:
   ```cmd
   cd app
   cargo build --release
   ```
4. The compiled executable will be located at `app/target/release/app.exe`. Copy it to the project root as `PotatoAI.exe`.

---

## Configuration

Settings are saved in `potato_config.json`:

```json
{
  "active_model": "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf",
  "temperature": 0.3,
  "system_prompt": "You are a helpful, direct, and concise AI assistant. Provide clean, correct, working answers directly.",
  "cpu_threads": 4
}
```

---

## License

MIT License. See individual engine files for third-party component licenses.
