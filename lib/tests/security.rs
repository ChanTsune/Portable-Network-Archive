use libpna::*;

#[test]
fn hashed_password_debug_does_not_leak_key() {
    let hp = HashedPassword::new(b"secret", HashAlgorithm::argon2id()).unwrap();
    let debug = format!("{hp:?}");
    assert!(debug.contains("[REDACTED]"), "Debug must redact key");
    assert!(!debug.contains("secret"), "Debug must not contain password");
}

#[test]
fn read_options_debug_does_not_leak_password() {
    let opts = ReadOptions::with_password(Some("secret"));
    let debug = format!("{opts:?}");
    assert!(!debug.contains("secret"), "Debug must not contain password");
}

#[test]
fn write_options_hashed_round_trip_no_panic() {
    let hashed = HashedPassword::new(b"password", HashAlgorithm::argon2id()).unwrap();
    let opts = WriteOptions::builder()
        .encryption(Encryption::Aes)
        .hashed_password(&hashed)
        .build();
    let _rebuilt = opts.into_builder().build();
}

#[test]
fn write_options_raw_round_trip_no_panic() {
    let opts = WriteOptions::builder()
        .encryption(Encryption::Aes)
        .password(Some("password"))
        .build();
    let _rebuilt = opts.into_builder().build();
}
