//! Retry con exponential backoff para llamadas LLM Ollama del coach.
//! Útil cuando Ollama está saturado o el modelo tarda más de lo esperado.

use std::future::Future;
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_total_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            max_total_ms: 25_000,
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn with_backoff<F, Fut, T, E>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let start = Instant::now();
    let mut backoff_ms = config.initial_backoff_ms;
    let mut last_err: Option<E> = None;

    for attempt in 1..=config.max_attempts {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        if elapsed_ms >= config.max_total_ms {
            log::warn!(
                "[retry/{}] excedido max_total_ms={} en intento {}",
                operation_name,
                config.max_total_ms,
                attempt
            );
            break;
        }

        match operation(attempt).await {
            Ok(v) => {
                if attempt > 1 {
                    log::info!(
                        "[retry/{}] OK tras {} intentos",
                        operation_name,
                        attempt
                    );
                }
                return Ok(v);
            }
            Err(e) => {
                log::warn!(
                    "[retry/{}] intento {} falló: {}",
                    operation_name,
                    attempt,
                    e
                );
                last_err = Some(e);
                if attempt < config.max_attempts {
                    let remaining = config.max_total_ms.saturating_sub(elapsed_ms);
                    let sleep_ms = backoff_ms.min(remaining.saturating_sub(100));
                    if sleep_ms == 0 {
                        break;
                    }
                    sleep(Duration::from_millis(sleep_ms)).await;
                    backoff_ms = (backoff_ms as f64 * config.backoff_multiplier) as u64;
                }
            }
        }
    }

    Err(last_err.expect("retry: no error captured but no success"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn succeeds_on_first_try() {
        let config = RetryConfig::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<i32, String> = with_backoff(&config, "test", |_| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_until_success() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 10,
            max_total_ms: 5000,
            backoff_multiplier: 2.0,
        };
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<i32, String> = with_backoff(&config, "test", |attempt| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                if attempt < 3 {
                    Err::<i32, String>("transient".to_string())
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_backoff_ms: 5,
            max_total_ms: 5000,
            backoff_multiplier: 2.0,
        };
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<(), String> = with_backoff(&config, "test", |_| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<(), String>("always fail".to_string())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
