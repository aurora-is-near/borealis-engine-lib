use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::signal::{
    ctrl_c,
    unix::{SignalKind, signal},
};
use tracing::info;

static SIGNAL: AtomicUsize = AtomicUsize::new(0);

const CUSTOM_CTRL_C_SIGNAL: usize = 602437500;

/// Tests a `signal_value` representing a system signal
/// to ensure it matches one of the following values: 602437500 | 15 | 12 | 10 | 1
pub fn is_matching_signal() -> bool {
    let signal_value = SIGNAL.load(Ordering::SeqCst) as i32;
    let signal_value = SignalKind::from(signal_value);

    match signal_value {
        // CUSTOM_CTRL_C_SIGNAL (602437500)
        val if val == SignalKind::from(CUSTOM_CTRL_C_SIGNAL as i32) => true,
        // TERM (15)
        val if val == SignalKind::terminate() => true,
        // USR2 (12)
        val if val == SignalKind::from_raw(12) => true,
        // USR1 (10)
        val if val == SignalKind::from_raw(10) => true,
        // SIGHUP (1)
        val if val == SignalKind::from_raw(1) => true,
        _ => false,
    }
}

/// Creates a future that handles USR1 signal and sends shutdown signal when triggered
pub async fn handle_usr1(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut kill_signal_stream = signal(SignalKind::from_raw(10))?;
    info!("Kill signal (USR1) handler installed");
    while kill_signal_stream.recv().await == Some(()) {
        info!("Kill signal (USR1) handler triggered");
        SIGNAL.store(10, Ordering::SeqCst);
        shutdown_tx
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
    }
    Ok(())
}

/// Creates a future that handles USR2 signal and sends shutdown signal when triggered
pub async fn handle_usr2(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut kill_signal_stream = signal(SignalKind::from_raw(12))?;
    info!("Kill signal (USR2) handler installed");
    while kill_signal_stream.recv().await == Some(()) {
        info!("Kill signal (USR2) handler triggered");
        SIGNAL.store(12, Ordering::SeqCst);
        shutdown_tx
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
    }
    Ok(())
}

/// Creates a future that handles TERM signal and sends shutdown signal when triggered
pub async fn handle_term(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut term_signal_stream = signal(SignalKind::terminate())?;
    info!("Terminate signal handler installed");
    while term_signal_stream.recv().await == Some(()) {
        info!("Terminate signal handler triggered");
        SIGNAL.store(15, Ordering::SeqCst);
        shutdown_tx
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
    }
    Ok(())
}

/// Creates a future that handles HUP signal and sends shutdown signal when triggered
pub async fn handle_hup(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut hup_signal_stream = signal(SignalKind::hangup())?;
    info!("Hangup signal handler installed");
    while hup_signal_stream.recv().await == Some(()) {
        info!("Hangup signal handler triggered");
        SIGNAL.store(1, Ordering::SeqCst);
        shutdown_tx
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
    }
    Ok(())
}

/// Creates a future that handles Ctrl+C and sends shutdown signal when triggered
pub async fn handle_ctrl_c(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    info!("Ctrl-C key sequence handler installed");
    while matches!(ctrl_c().await, Ok(())) {
        info!("Ctrl-C key sequence handler triggered");
        SIGNAL.store(602437500, Ordering::SeqCst);
        shutdown_tx
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
    }
    Ok(())
}

/// Creates a future that handles all signals and sends shutdown signal when any is triggered
pub async fn handle_all_signals(
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
) -> anyhow::Result<()> {
    let result = tokio::select! {
        result = handle_usr1(shutdown_tx.clone()) => result,
        result = handle_usr2(shutdown_tx.clone()) => result,
        result = handle_term(shutdown_tx.clone()) => result,
        result = handle_hup(shutdown_tx.clone()) => result,
        result = handle_ctrl_c(shutdown_tx) => result,
    };
    actix::System::current().stop();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_matching_signal() {
        SIGNAL.store(2, Ordering::SeqCst);
        assert!(!is_matching_signal());

        SIGNAL.store(1, Ordering::SeqCst);
        assert!(is_matching_signal());

        SIGNAL.store(10, Ordering::SeqCst);
        assert!(is_matching_signal());

        SIGNAL.store(12, Ordering::SeqCst);
        assert!(is_matching_signal());

        SIGNAL.store(15, Ordering::SeqCst);
        assert!(is_matching_signal());

        SIGNAL.store(602437500, Ordering::SeqCst);
        assert!(is_matching_signal());
    }
}
