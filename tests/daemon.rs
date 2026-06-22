//! End-to-end check of the daemon manager against the bundled arduino-cli
//! v1.5.1 binary. Offline-safe: `Init` only reads the local data dir. Requires
//! the per-platform binary under `arduino-cli-binaries/` to be present.

use thingblock_link::daemon::Daemon;

#[tokio::test]
async fn starts_daemon_and_completes_handshake() {
    let daemon = Daemon::start()
        .await
        .expect("daemon should spawn and complete Create/Init handshake");

    assert_ne!(
        daemon.instance().id,
        0,
        "Init should yield a non-zero instance id"
    );
    // Dropping `daemon` kills the child via kill_on_drop.
}
