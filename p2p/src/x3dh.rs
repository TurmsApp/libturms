use error::{Error, Result};
use tokio::time::{Duration, sleep};
use tracing::{debug, error};
use vodozemac::olm::Account;

use crate::ACCOUNT;
use crate::models::Event::DHKey;
use crate::models::X3DH;

use std::sync::Mutex;

const MAX_ATTEMPTS: u8 = 4;

/// Spawn triple diffie hellman generation logic over WebRTC.
pub async fn triple_diffie_hellman(
    acc: &crate::webrtc::WebRTCManager,
) -> Result<()> {
    if acc.is_initiator {
        return Ok(());
    }

    let account = ACCOUNT.get_or_init(|| Mutex::new(Account::new()));

    // Generate public key and one-time key.
    let (public_key, otk) = {
        let mut account = match account.lock() {
            Ok(a) => a,
            Err(_) => {
                error!("account mutex is poisoned");
                return Err(Error::MutexPoisoned);
            },
        };

        account.generate_one_time_keys(1);

        let public_key = account.curve25519_key().to_vec();
        let otk = match account.one_time_keys().values().next() {
            Some(k) => k.to_vec(),
            None => {
                // Since insertion occurs before, this should never happen here.
                return Err(Error::MutexPoisoned);
            },
        };

        (public_key, otk)
    };

    let message = serde_json::to_string(&DHKey(X3DH { public_key, otk }))?;

    if let Some(ch) = acc.channel.as_ref() {
        for n in 0..MAX_ATTEMPTS {
            if n > 0 {
                // Wait only if first try is failed.
                sleep(Duration::from_secs(u64::from(n) * 5)).await;
            }

            match ch.send_text(&message).await {
                Ok(_) => {
                    if let Ok(mut acc_lock) = account.lock() {
                        acc_lock.mark_keys_as_published();
                        debug!("public key and one-time key published");
                    }
                    break;
                },
                Err(err) => {
                    error!(%err, "{n}th attempt to send keys to a peer failed");
                    if n == MAX_ATTEMPTS - 1 {
                        return Err(Error::MessageSendFailed);
                    }
                },
            }
        }
    }

    Ok(())
}
