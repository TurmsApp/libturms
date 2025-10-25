//! peer-to-peer communication via WebRTC.
/// Models.
pub mod models;
#[forbid(unsafe_code)]
#[deny(missing_docs, missing_debug_implementations)]
/// WebRTC interface.
pub mod webrtc;
/// X3DH over WebRTC for Turms.
mod x3dh;

use std::sync::{Mutex, OnceLock};

static ACCOUNT: OnceLock<Mutex<vodozemac::olm::Account>> = OnceLock::new();

/// Get user account.
pub fn save_account() -> error::Result<String> {
    let account =
        ACCOUNT.get_or_init(|| Mutex::new(vodozemac::olm::Account::new()));

    Ok(serde_json::to_string(
        &account
            .lock()
            .map_err(|_| error::Error::MutexPoisoned)?
            .pickle(),
    )?)
}

/// Set user account.
pub fn restore_account(json: &str) -> Result<(), serde_json::Error> {
    let pickle: vodozemac::olm::AccountPickle = serde_json::from_str(json)?;

    let _ = ACCOUNT.get_or_init(|| {
        Mutex::new(vodozemac::olm::Account::from_pickle(pickle))
    });

    Ok(())
}
