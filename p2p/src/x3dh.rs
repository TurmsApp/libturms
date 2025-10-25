pub const MAX_ATTEMPTS: u8 = 4;

/// Spawn triple diffie hellman generation logic over WebRTC.
#[macro_export]
macro_rules! triple_diffie_hellman {
    ($acc:expr) => {{
        use $crate::models::Event::DHKey;
        use $crate::ACCOUNT;
        use $crate::x3dh::MAX_ATTEMPTS;
        use tracing::{error, debug};
        use tokio::time::{sleep, Duration};

        if $acc.is_initiator {
            return;
        }

        let account =
            ACCOUNT.get_or_init(|| Mutex::new(Account::new()));

        // Generate public key and one-time key.
        let (public_key, otk) = {
            let mut account = match account.lock() {
                Ok(a) => a,
                Err(_) => {
                    error!("account mutex is poisoned");
                    return;
                },
            };

            account.generate_one_time_keys(1);

            let public_key = account.curve25519_key().to_vec();
            let otk = match account.one_time_keys().values().next() {
                Some(k) => k.to_vec(),
                None => {
                    // Since insertion occurs before, this should never happen here.
                    // If this is the case, it is best to restart. Let it crash. Poison Mutex.
                    unreachable!()
                },
            };

            (public_key, otk)
        };

        let message = match serde_json::to_string(&DHKey(X3DH {
            public_key,
            otk,
        })) {
            Ok(message) => message,
            Err(err) => {
                error!(%err, "serialization failed");
                return;
            },
        };

        if let Some(ch) = $acc.channel.as_ref() {
            for n in 0..MAX_ATTEMPTS {
                if n > 0 {
                    // Wait only if first try is failed.
                    sleep(Duration::from_secs(u64::from(n) * 5)).await;
                }

                match ch.send_text(&message).await {
                    Ok(_) => {
                        if let Ok(mut acc_lock) = account.lock() {
                            acc_lock.mark_keys_as_published();
                            debug!("public key and one-time key published")
                        }
                        break;
                    },
                    Err(err) => error!(%err, "{n}th attempt to send keys to a peer failed"),
                }
            }
        }
    }};
}
