use std::io::{self, BufRead, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use encoding_rs;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use serde::{Deserialize, Serialize};

// ============================================================================
// Protocol Messages (JSON over stdin/stdout)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Generate {
        prompt: String,
        max_tokens: Option<i32>,
        context_size: Option<u32>,
        model_path: Option<String>,
        // Sampling parameters
        temperature: Option<f32>,
        top_k: Option<i32>,
        top_p: Option<f32>,
        stop_tokens: Option<Vec<String>>,
    },
    /// Streaming variant — emits one `Token` per token, then `StreamDone`.
    GenerateStream {
        prompt: String,
        max_tokens: Option<i32>,
        context_size: Option<u32>,
        model_path: Option<String>,
        temperature: Option<f32>,
        top_k: Option<i32>,
        top_p: Option<f32>,
        stop_tokens: Option<Vec<String>>,
    },
    Ping,
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Response { text: String, error: Option<String> },
    /// Streaming token (one per generated piece of text).
    Token { text: String },
    /// Stream completion marker (with optional error if generation failed).
    StreamDone { error: Option<String> },
    Pong,
    Goodbye,
    Error { message: String },
}

// ============================================================================
// VRAM Detection and GPU Layer Calculation
// ============================================================================

/// Detect available VRAM in GB
fn detect_vram_gb() -> f32 {
    #[cfg(feature = "metal")]
    {
        // macOS Metal: Query recommended max working set size
        if let Some(vram) = detect_metal_vram() {
            eprintln!("Metal VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    #[cfg(feature = "cuda")]
    {
        // NVIDIA CUDA: Query device memory
        if let Some(vram) = detect_cuda_vram() {
            eprintln!("CUDA VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    /// TODO: Vulkan VRAM detection

    eprintln!("VRAM detection not available, using conservative estimate");
    4.0 // Conservative fallback
}

#[cfg(feature = "metal")]
fn detect_metal_vram() -> Option<f32> {
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("hw.memsize")
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Some(bytes_str) = stdout.split(':').nth(1) {
                if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                    let gb = bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    // Assume GPU can use ~60% of system memory on Apple Silicon
                    return Some(gb * 0.6);
                }
            }
        }
    }
    None
}

#[cfg(feature = "cuda")]
fn detect_cuda_vram() -> Option<f32> {
    // Use nvidia-smi to query VRAM
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=memory.free", "--format=csv,noheader,nounits"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Ok(mb) = stdout.trim().parse::<f32>() {
                return Some(mb / 1024.0); // Convert MB to GB
            }
        }
    }
    None
}

/// Calculate safe GPU layer count based on VRAM, model file size, and context size
fn calculate_gpu_layers(
    model_path: &PathBuf,
    model_layers: u32,
    vram_gb: f32,
    context_size: u32,
) -> u32 {
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    if file_size_gb == 0.0 {
        eprintln!("⚠️ Could not determine model file size, using conservative default");
        return 0;
    }

    // Heuristic: Estimate KV cache size
    // 7B models (approx > 2.5GB) usually have 4096 hidden dim -> ~256MB per 1k context
    // 1B models (approx < 2.5GB) usually have 2048 hidden dim -> ~128MB per 1k context
    let kv_per_1k_gb = if file_size_gb > 2.5 { 0.25 } else { 0.12 };
    let total_kv_gb = (context_size as f32 / 1000.0) * kv_per_1k_gb;

    // Safety buffer (500MB) for OS/Display
    let safe_vram = vram_gb - 0.5;

    // For debugging
    eprintln!("📊 VRAM Analysis:");
    eprintln!("   • Available: {:.2} GB", vram_gb);
    eprintln!("   • Safe Limit: {:.2} GB", safe_vram);
    eprintln!("   • Model Weights: {:.2} GB", file_size_gb);
    eprintln!(
        "   • KV Cache ({} ctx): {:.2} GB",
        context_size, total_kv_gb
    );

    if safe_vram <= 0.0 {
        eprintln!("⚠️ No safe VRAM available, using CPU only");
        return 0;
    }

    // Calculate cost per layer
    let weight_per_layer = file_size_gb / model_layers as f32;
    let kv_per_layer = total_kv_gb / model_layers as f32;
    let total_per_layer = weight_per_layer + kv_per_layer;

    // Calculate how many layers fit
    let safe_layers = (safe_vram / total_per_layer).floor() as u32;
    let layers = safe_layers.min(model_layers);

    eprintln!(
        "   • Cost per layer: {:.2} MB (Weights) + {:.2} MB (KV) = {:.2} MB",
        weight_per_layer * 1024.0,
        kv_per_layer * 1024.0,
        total_per_layer * 1024.0
    );

    if layers < model_layers {
        eprintln!(
            "⚠️ Memory constrained. Offloading {}/{} layers ({:.1}%)",
            layers,
            model_layers,
            (layers as f32 / model_layers as f32) * 100.0
        );
    } else {
        eprintln!("✅ Full offload possible ({} layers)", layers);
    }

    layers
}

/// Get default GPU layer count with smart detection
fn get_default_gpu_layers(model_path: &PathBuf, context_size: u32) -> u32 {
    let vram = detect_vram_gb();
    // TODO: Use actual model metadata instead of heuristics
    // Heuristic: Estimate total layers based on file size
    // 7B models (Q4) are ~4.1GB and have ~32-35 layers
    // 1B models (Q4) are ~1.1GB and have ~20-28 layers
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    let estimated_layers = if file_size_gb > 2.5 { 33 } else { 28 };

    calculate_gpu_layers(model_path, estimated_layers, vram, context_size)
}

// ============================================================================
// Model State Management
// ============================================================================

struct ModelState {
    backend: LlamaBackend,
    model: Option<LlamaModel>,
    model_path: Option<PathBuf>,
    context_size: u32,
    last_activity: Arc<AtomicU64>,
}

impl ModelState {
    fn new() -> Result<Self> {
        let backend = LlamaBackend::init().context("Failed to init LlamaBackend")?;
        Ok(Self {
            backend,
            model: None,
            model_path: None,
            context_size: 2048,
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
        })
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn update_activity(&self) {
        self.last_activity
            .store(Self::current_timestamp(), Ordering::SeqCst);
    }

    fn seconds_since_activity(&self) -> u64 {
        Self::current_timestamp() - self.last_activity.load(Ordering::SeqCst)
    }

    fn load_model_if_needed(&mut self, model_path: PathBuf, context_size: u32) -> Result<()> {
        // Check if model is already loaded
        if let Some(ref loaded_path) = self.model_path {
            if loaded_path == &model_path && self.context_size == context_size {
                eprintln!("✓ Model already loaded");
                self.update_activity();
                return Ok(());
            }
        }

        eprintln!("📥 Loading model: {}", model_path.display());

        // Detect GPU layers
        let gpu_layers = get_default_gpu_layers(&model_path, context_size);

        // Configure model parameters with GPU offload
        let model_params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
        let model_params = pin!(model_params);

        let model = LlamaModel::load_from_file(&self.backend, model_path.clone(), &model_params)
            .with_context(|| format!("unable to load model at {:?}", model_path))?;

        self.model = Some(model);
        self.model_path = Some(model_path);
        self.context_size = context_size;
        self.update_activity();

        eprintln!("✅ Model loaded successfully");
        Ok(())
    }

    fn generate(
        &mut self,
        prompt: String,
        max_tokens: i32,
        temperature: f32,
        top_k: i32,
        top_p: f32,
        stop_tokens: Vec<String>,
    ) -> Result<String> {
        let start_time = Instant::now();
        let model = self.model.as_ref().context("Model not loaded")?;

        // n_threads = cores - 1 (leave 1 core for UI/OS).
        // Anterior `(cores/2) + 2` subutilizaba CPUs ≥ 6 cores; en Ryzen 9 8c
        // pasamos de 6 → 7 threads (+15-20% throughput según perf-oracle).
        let threads: i32 = std::thread::available_parallelism()
            .map(|n| {
                let cores = n.get() as i32;
                (cores - 1).max(1)
            })
            .unwrap_or(2);

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(
                NonZeroU32::new(self.context_size).context("Invalid ctx size")?,
            ))
            .with_n_batch(self.context_size)
            .with_n_threads(threads)
            .with_n_threads_batch(threads);

        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .context("unable to create the llama_context")?;

        let tokens_list = model
            .str_to_token(&prompt, AddBos::Always)
            .with_context(|| "failed to tokenize prompt")?;

        eprintln!("📝 Tokenized prompt: {} tokens", tokens_list.len());

        // Use context size for batch capacity to handle long prompts
        let batch_size = self.context_size as usize;
        let mut batch = LlamaBatch::new(batch_size, 1);

        let last_index: i32 = (tokens_list.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            let is_last = i == last_index;
            batch
                .add(token, i, &[0], is_last)
                .context("Failed to add token to batch")?;
        }

        ctx.decode(&mut batch).context("llama_decode() failed")?;
        let prompt_time = start_time.elapsed();

        let n_prompt_tokens = batch.n_tokens();
        let mut n_cur = n_prompt_tokens;
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();

        eprintln!("🔄 Starting generation (max_tokens: {})", max_tokens);

        loop {
            // Check if we've generated enough tokens
            if (n_cur - n_prompt_tokens) >= max_tokens {
                eprintln!("✓ Reached max_tokens limit");
                break;
            }

            use llama_cpp_2::sampling::LlamaSampler;

            let sampler = if temperature <= 0.0 {
                // Greedy sampling for temp <= 0
                LlamaSampler::chain_simple([LlamaSampler::greedy()])
            } else {
                // Random sampling with temperature/top_k/top_p
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u32;

                LlamaSampler::chain_simple([
                    LlamaSampler::top_k(top_k),
                    LlamaSampler::top_p(top_p, 1),
                    LlamaSampler::temp(temperature),
                    LlamaSampler::dist(seed),
                ])
            };

            let mut sampler = pin!(sampler);
            let token = sampler.as_mut().sample(&ctx, batch.n_tokens() - 1);
            sampler.as_mut().accept(token);

            if model.is_eog_token(token) {
                eprintln!(
                    "✓ End-of-generation token reached (generated {} chars)",
                    output.len()
                );
                break;
            }

            let output_bytes = model
                .token_to_bytes(token, Special::Tokenize)
                .context("Failed to convert token to bytes")?;

            let mut token_text = String::with_capacity(32);
            let _ = decoder.decode_to_string(&output_bytes, &mut token_text, false);
            output.push_str(&token_text);

            // Check for model-specific stop tokens
            let mut should_stop = false;
            for stop_token in &stop_tokens {
                if output.contains(stop_token) {
                    eprintln!(
                        "✓ Stop token '{}' detected (generated {} chars)",
                        stop_token,
                        output.len()
                    );
                    // Remove the stop token from output
                    output = output.replace(stop_token, "").trim_end().to_string();
                    should_stop = true;
                    break;
                }
            }
            if should_stop {
                break;
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .context("Failed to add generated token to batch")?;
            n_cur += 1;
            ctx.decode(&mut batch).context("failed to eval")?;
        }

        // Generation statistics
        let total_time = start_time.elapsed();
        let gen_time = total_time.saturating_sub(prompt_time);
        let output_tokens = (n_cur - n_prompt_tokens) as u64;
        let prompt_tokens = n_prompt_tokens as u64;

        let tokens_per_sec = if gen_time.as_secs_f64() > 0.0 {
            output_tokens as f64 / gen_time.as_secs_f64()
        } else {
            0.0
        };

        eprintln!("📊 Generation Statistics:");
        eprintln!("   • Prompt tokens: {}", prompt_tokens);
        eprintln!("   • Output tokens: {}", output_tokens);
        eprintln!("   • Prompt processing: {:.2}s", prompt_time.as_secs_f64());
        eprintln!("   • Generation time: {:.2}s", gen_time.as_secs_f64());
        eprintln!("   • Total time: {:.2}s", total_time.as_secs_f64());
        eprintln!("   • Speed: {:.2} tokens/sec", tokens_per_sec);

        self.update_activity();
        Ok(output)
    }

    /// Streaming variant: emite cada token via `emit_token` as soon as decoded.
    /// Returns `Ok(())` on normal completion (EOG / max_tokens / stop_token).
    fn generate_stream<F>(
        &mut self,
        prompt: String,
        max_tokens: i32,
        temperature: f32,
        top_k: i32,
        top_p: f32,
        stop_tokens: Vec<String>,
        mut emit_token: F,
    ) -> Result<()>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let start_time = Instant::now();
        let model = self.model.as_ref().context("Model not loaded")?;

        let threads: i32 = std::thread::available_parallelism()
            .map(|n| {
                let cores = n.get() as i32;
                (cores - 1).max(1)
            })
            .unwrap_or(2);

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(
                NonZeroU32::new(self.context_size).context("Invalid ctx size")?,
            ))
            .with_n_batch(self.context_size)
            .with_n_threads(threads)
            .with_n_threads_batch(threads);

        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .context("unable to create the llama_context")?;

        let tokens_list = model
            .str_to_token(&prompt, AddBos::Always)
            .with_context(|| "failed to tokenize prompt")?;

        eprintln!("📝 [stream] Tokenized prompt: {} tokens", tokens_list.len());

        let batch_size = self.context_size as usize;
        let mut batch = LlamaBatch::new(batch_size, 1);

        let last_index: i32 = (tokens_list.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            let is_last = i == last_index;
            batch
                .add(token, i, &[0], is_last)
                .context("Failed to add token to batch")?;
        }

        ctx.decode(&mut batch).context("llama_decode() failed")?;
        let prompt_time = start_time.elapsed();

        let n_prompt_tokens = batch.n_tokens();
        let mut n_cur = n_prompt_tokens;
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        // Acumulado para detección multi-token de stop sequences.
        let mut buffered_tail = String::new();

        eprintln!("🔄 [stream] Starting generation (max_tokens: {})", max_tokens);

        loop {
            if (n_cur - n_prompt_tokens) >= max_tokens {
                eprintln!("✓ [stream] Reached max_tokens limit");
                break;
            }

            use llama_cpp_2::sampling::LlamaSampler;

            let sampler = if temperature <= 0.0 {
                LlamaSampler::chain_simple([LlamaSampler::greedy()])
            } else {
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u32;

                LlamaSampler::chain_simple([
                    LlamaSampler::top_k(top_k),
                    LlamaSampler::top_p(top_p, 1),
                    LlamaSampler::temp(temperature),
                    LlamaSampler::dist(seed),
                ])
            };

            let mut sampler = pin!(sampler);
            let token = sampler.as_mut().sample(&ctx, batch.n_tokens() - 1);
            sampler.as_mut().accept(token);

            if model.is_eog_token(token) {
                eprintln!("✓ [stream] EOG token reached");
                break;
            }

            let output_bytes = model
                .token_to_bytes(token, Special::Tokenize)
                .context("Failed to convert token to bytes")?;

            let mut token_text = String::with_capacity(32);
            let _ = decoder.decode_to_string(&output_bytes, &mut token_text, false);

            // Stop-token detection: keep a small tail buffer (max len of any stop token + token_text)
            // to detect cross-token stop sequences. If found, truncate and stop.
            buffered_tail.push_str(&token_text);
            let max_stop_len = stop_tokens.iter().map(|s| s.len()).max().unwrap_or(0);
            let mut hit_stop = false;
            let mut emit_text = token_text.clone();
            // Stop-token detection en buffered_tail (ya incluye token_text).
            // Si el stop arranca DENTRO de token_text actual, emitimos solo el
            // prefijo seguro previo al stop. Si arranca antes (en tokens previos
            // ya emitidos) simplemente cortamos sin re-emitir.
            for stop_token in &stop_tokens {
                if let Some(cut) = buffered_tail.find(stop_token) {
                    let prev_tail_len = buffered_tail.len() - token_text.len();
                    if cut >= prev_tail_len {
                        // Cantidad de bytes del token_text actual ANTES del stop.
                        let bytes_before_stop = cut - prev_tail_len;
                        // Avanza al char-boundary <= bytes_before_stop (UTF-8 safe).
                        let safe_end = (0..=bytes_before_stop)
                            .rev()
                            .find(|i| token_text.is_char_boundary(*i))
                            .unwrap_or(0);
                        emit_text = token_text[..safe_end].to_string();
                    } else {
                        emit_text.clear();
                    }
                    eprintln!("✓ [stream] Stop token '{}' detected", stop_token);
                    hit_stop = true;
                    break;
                }
            }

            if !emit_text.is_empty() {
                emit_token(&emit_text)?;
            }

            // Trim tail buffer to bounded size — UTF-8 safe (drain on char boundary).
            let bound = max_stop_len.saturating_mul(2).saturating_add(64);
            if buffered_tail.len() > bound {
                let target_len = max_stop_len.saturating_add(32);
                let drain_bytes = buffered_tail.len().saturating_sub(target_len);
                // Avanza hasta el siguiente char boundary >= drain_bytes.
                let safe_split = (drain_bytes..=buffered_tail.len())
                    .find(|i| buffered_tail.is_char_boundary(*i))
                    .unwrap_or(buffered_tail.len());
                buffered_tail.drain(..safe_split);
            }

            if hit_stop {
                break;
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .context("Failed to add generated token to batch")?;
            n_cur += 1;
            ctx.decode(&mut batch).context("failed to eval")?;
        }

        let total_time = start_time.elapsed();
        let gen_time = total_time.saturating_sub(prompt_time);
        let output_tokens = (n_cur - n_prompt_tokens) as u64;
        let tokens_per_sec = if gen_time.as_secs_f64() > 0.0 {
            output_tokens as f64 / gen_time.as_secs_f64()
        } else {
            0.0
        };

        eprintln!(
            "📊 [stream] Done — {} tok in {:.2}s ({:.2} tok/s, prefill {:.2}s)",
            output_tokens,
            gen_time.as_secs_f64(),
            tokens_per_sec,
            prompt_time.as_secs_f64()
        );

        self.update_activity();
        Ok(())
    }
}

// ============================================================================
// Main Loop with Keep-Alive Protocol
// ============================================================================

fn send_response(response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Get idle timeout from environment variable (default 5 minutes)
    let idle_timeout_secs = std::env::var("LLAMA_IDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300); // 5 minutes default

    eprintln!(
        "🦙 llama-helper starting (idle timeout: {}s)",
        idle_timeout_secs
    );

    let mut state = ModelState::new()?;

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buffer = String::new();

    loop {
        // Check idle timeout
        if state.seconds_since_activity() > idle_timeout_secs {
            eprintln!("💤 Idle timeout reached, shutting down");
            send_response(&Response::Goodbye)?;
            break;
        }

        // Read line from stdin
        buffer.clear();
        match stdin_lock.read_line(&mut buffer) {
            Ok(0) => {
                // EOF reached
                eprintln!("📪 EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                // Parse request
                match serde_json::from_str::<Request>(line) {
                    Ok(Request::Generate {
                        prompt,
                        max_tokens,
                        context_size,
                        model_path,
                        temperature,
                        top_k,
                        top_p,
                        stop_tokens,
                    }) => {
                        let max_tokens = max_tokens.unwrap_or(512);
                        let context_size = context_size.unwrap_or(2048);

                        // Sampling parameters with sensible defaults
                        let temperature = temperature.unwrap_or(1.0);
                        let top_k = top_k.unwrap_or(64);
                        let top_p = top_p.unwrap_or(0.95);
                        let stop_tokens = stop_tokens.unwrap_or_else(Vec::new);

                        // Load model if path provided
                        if let Some(path_str) = model_path {
                            let path = PathBuf::from(path_str);
                            if let Err(e) = state.load_model_if_needed(path, context_size) {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Failed to load model: {}", e)),
                                })?;
                                continue;
                            }
                        }

                        // Generate response with sampling parameters
                        match state.generate(
                            prompt,
                            max_tokens,
                            temperature,
                            top_k,
                            top_p,
                            stop_tokens,
                        ) {
                            Ok(text) => {
                                send_response(&Response::Response { text, error: None })?;
                            }
                            Err(e) => {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Generation failed: {}", e)),
                                })?;
                            }
                        }
                    }
                    Ok(Request::GenerateStream {
                        prompt,
                        max_tokens,
                        context_size,
                        model_path,
                        temperature,
                        top_k,
                        top_p,
                        stop_tokens,
                    }) => {
                        let max_tokens = max_tokens.unwrap_or(512);
                        let context_size = context_size.unwrap_or(2048);
                        let temperature = temperature.unwrap_or(1.0);
                        let top_k = top_k.unwrap_or(64);
                        let top_p = top_p.unwrap_or(0.95);
                        let stop_tokens = stop_tokens.unwrap_or_else(Vec::new);

                        if let Some(path_str) = model_path {
                            let path = PathBuf::from(path_str);
                            if let Err(e) = state.load_model_if_needed(path, context_size) {
                                send_response(&Response::StreamDone {
                                    error: Some(format!("Failed to load model: {}", e)),
                                })?;
                                continue;
                            }
                        }

                        let result = state.generate_stream(
                            prompt,
                            max_tokens,
                            temperature,
                            top_k,
                            top_p,
                            stop_tokens,
                            |chunk| {
                                send_response(&Response::Token {
                                    text: chunk.to_string(),
                                })
                            },
                        );

                        match result {
                            Ok(()) => {
                                send_response(&Response::StreamDone { error: None })?;
                            }
                            Err(e) => {
                                send_response(&Response::StreamDone {
                                    error: Some(format!("Generation failed: {}", e)),
                                })?;
                            }
                        }
                    }
                    Ok(Request::Ping) => {
                        state.update_activity();
                        send_response(&Response::Pong)?;
                    }
                    Ok(Request::Shutdown) => {
                        eprintln!("🛑 Shutdown requested");
                        send_response(&Response::Goodbye)?;
                        break;
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to parse request: {}", e);
                        send_response(&Response::Error {
                            message: format!("Invalid request: {}", e),
                        })?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading stdin: {}", e);
                break;
            }
        }
    }

    eprintln!("👋 llama-helper exiting");
    Ok(())
}
