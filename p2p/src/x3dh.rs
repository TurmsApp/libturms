use error::{Error, Result};

use crate::models::Event::DHKey;
use crate::models::X3DH;

/// Spawn triple diffie hellman generation logic over WebRTC.
pub async fn triple_diffie_hellman(
    acc: &crate::webrtc::WebRTCManager,
) -> Result<()> {
    if acc.is_initiator {
        return Ok(());
    }

    let account = crate::get_account();

    // Generate public key and one-time key.
    let (public_key, otk) = {
        let mut account = account.lock().await;

        account.generate_one_time_keys(1);

        let public_key = account.curve25519_key();
        let otk = match account.one_time_keys().values().next() {
            Some(k) => *k,
            None => {
                // Since insertion occurs before, this should never happen here.
                return Err(Error::MutexPoisoned);
            },
        };

        (public_key, Some(otk))
    };

    let message = serde_json::to_string(&DHKey(X3DH {
        public_key,
        otk,
        prekey: None,
    }))?;

    if acc.send(message).await.is_ok() {
        account.lock().await.mark_keys_as_published();
        tracing::debug!("public key and one-time key published");
    };

    Ok(())
}
