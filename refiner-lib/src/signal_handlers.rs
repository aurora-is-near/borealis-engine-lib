use std::sync::atomic::{AtomicI32, Ordering};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::broadcast::error::SendError;
use tracing::{info, warn};

static SIGNAL: AtomicI32 = AtomicI32::new(0);

static SIGNAL_USR1: i32 = 10;
static SIGNAL_USR2: i32 = 12;

/// Tests a `signal_value` representing a system signal
/// to ensure it matches one of the following values: 2 | 15 | 12 | 10 | 1
pub fn is_matching_signal() -> bool {
    let signal_value: SignalKind = SIGNAL.load(Ordering::SeqCst).into();

    match signal_value {
        // SIGINT (2)
        val if val == SignalKind::interrupt() => true,
        // TERM (15)
        val if val == SignalKind::terminate() => true,
        // USR2 (12)
        val if val == SignalKind::from_raw(SIGNAL_USR2) => true,
        // USR1 (10)
        val if val == SignalKind::from_raw(SIGNAL_USR1) => true,
        // SIGHUP (1)
        val if val == SignalKind::hangup() => true,
        _ => false,
    }
}

/// Creates a future that handles SIGQUIT signal and sends shutdown signal when triggered
pub async fn handle_quit(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let sigquit = SignalKind::quit();
    let mut quit_signal_stream = signal(sigquit)?;
    info!("Signal handler for SIGQUIT installed");

    quit_signal_stream.recv().await;

    info!("Signal handler for SIGQUIT triggered");
    SIGNAL.store(sigquit.into(), Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by SIGQUIT shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by SIGQUIT"),
    }

    Ok(())
}

/// Creates a future that handles USR1 signal and sends shutdown signal when triggered
pub async fn handle_usr1(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut kill_signal_stream = signal(SignalKind::from_raw(SIGNAL_USR1))?;
    info!("Custom signal handler for SIGUSR1 {SIGNAL_USR1} installed");

    kill_signal_stream.recv().await;

    info!("Custom signal handler for SIGUSR1 {SIGNAL_USR1} triggered");
    SIGNAL.store(SIGNAL_USR1, Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by SIGUSR1 shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by SIGUSR1"),
    }

    Ok(())
}

/// Creates a future that handles USR2 signal and sends shutdown signal when triggered
pub async fn handle_usr2(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let mut kill_signal_stream = signal(SignalKind::from_raw(SIGNAL_USR2))?;
    info!("Custom signal handler for SIGUSR2 {SIGNAL_USR2} installed");

    kill_signal_stream.recv().await;

    info!("Custom signal handler for SIGUSR2 {SIGNAL_USR2} triggered");
    SIGNAL.store(SIGNAL_USR2, Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by SIGUSR2 shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by SIGUSR2"),
    }

    Ok(())
}

/// Creates a future that handles TERM signal and sends shutdown signal when triggered
pub async fn handle_term(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let sigterm = SignalKind::terminate();
    let mut term_signal_stream = signal(sigterm)?;
    info!("Signal handler for SIGTERM installed");

    term_signal_stream.recv().await;

    info!("Signal handler for SIGTERM triggered");
    SIGNAL.store(sigterm.into(), Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by SIGTERM shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by SIGTERM"),
    }

    Ok(())
}

/// Creates a future that handles HUP signal and sends shutdown signal when triggered
pub async fn handle_hup(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let sighup = SignalKind::hangup();
    let mut hup_signal_stream = signal(sighup)?;
    info!("Signal handler for SIGHUP installed");

    hup_signal_stream.recv().await;

    info!("Signal handler for SIGHUP triggered");
    SIGNAL.store(sighup.into(), Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by SIGHUP shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by SIGHUP"),
    }

    Ok(())
}

/// Creates a future that handles Ctrl+C and sends shutdown signal when triggered
pub async fn handle_ctrl_c(shutdown_tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    let sigint = SignalKind::interrupt();
    info!("Signal handler for Ctrl-C installed");

    tokio::signal::ctrl_c().await?;

    info!("Signal handler for Ctrl-C triggered");
    SIGNAL.store(sigint.into(), Ordering::SeqCst);
    match shutdown_tx.send(()) {
        Ok(_) => info!("Originated by Ctrl-C shutdown signal sent successfully"),
        Err(SendError(_)) => warn!("No active receivers for shutdown signal originated by Ctrl-C"),
    }

    Ok(())
}

/// Creates a future that handles all signals and sends shutdown signal when any is triggered
pub async fn handle_all_signals(
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
) -> anyhow::Result<()> {
    info!("Installing signal handlers");

    let mut usr1 = Box::pin(handle_usr1(shutdown_tx.clone()));
    let mut usr2 = Box::pin(handle_usr2(shutdown_tx.clone()));
    let mut term = Box::pin(handle_term(shutdown_tx.clone()));
    let mut hup = Box::pin(handle_hup(shutdown_tx.clone()));
    let mut ctrl_c = Box::pin(handle_ctrl_c(shutdown_tx));

    tokio::select! {
        result = &mut usr1 => {
            if let Err(e) = result {
                warn!("Error handling USR1 signal: {}", e);
            }
        }
        result = &mut usr2 => {
            if let Err(e) = result {
                warn!("Error handling USR2 signal: {}", e);
            }
        }
        result = &mut term => {
            if let Err(e) = result {
                warn!("Error handling TERM signal: {}", e);
            }
        }
        result = &mut hup => {
            if let Err(e) = result {
                warn!("Error handling HUP signal: {}", e);
            }
        }
        result = &mut ctrl_c => {
            if let Err(e) = result {
                warn!("Error handling Ctrl-C signal: {}", e);
            }
        }
    };

    info!("Stopping actix system");
    actix::System::current().stop();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_matching_signal() {
        // Non-matching value
        SIGNAL.store(602437500, Ordering::SeqCst);
        assert!(!is_matching_signal());

        // SIGINT (2)
        SIGNAL.store(SignalKind::interrupt().into(), Ordering::SeqCst);
        assert!(is_matching_signal());

        // SIGUSR1 (10)
        SIGNAL.store(SignalKind::from_raw(SIGNAL_USR1).into(), Ordering::SeqCst);
        assert!(is_matching_signal());

        // SIGUSR2 (12)
        SIGNAL.store(SignalKind::from_raw(SIGNAL_USR2).into(), Ordering::SeqCst);
        assert!(is_matching_signal());

        // TERM (15)
        SIGNAL.store(SignalKind::terminate().into(), Ordering::SeqCst);
        assert!(is_matching_signal());

        // SIGHUP (1)
        SIGNAL.store(SignalKind::hangup().into(), Ordering::SeqCst);
        assert!(is_matching_signal());
    }
}
