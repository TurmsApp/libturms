//! peer-to-peer communication via WebRTC.
/// Models.
pub mod models;
#[forbid(unsafe_code)]
#[deny(missing_docs, missing_debug_implementations)]
/// WebRTC interface.
pub mod webrtc;
/// X3DH over WebRTC for Turms.
mod x3dh;

pub use x3dh::triple_diffie_hellman;

use tokio::sync::Mutex;
use vodozemac::olm::Account;

use std::sync::OnceLock;

static ACCOUNT: OnceLock<Mutex<Account>> = OnceLock::new();

pub fn get_account() -> &'static Mutex<Account> {
    ACCOUNT.get_or_init(|| Mutex::new(Account::new()))
}

/// Get user account.
pub async fn save_account() -> error::Result<String> {
    let account = get_account();

    Ok(serde_json::to_string(&account.lock().await.pickle())?)
}

/// Set user account.
pub fn restore_account(json: &str) -> Result<(), serde_json::Error> {
    let pickle: vodozemac::olm::AccountPickle = serde_json::from_str(json)?;

    let _ = ACCOUNT.get_or_init(|| Mutex::new(Account::from_pickle(pickle)));

    Ok(())
}
