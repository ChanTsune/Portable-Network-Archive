use argon2::{password_hash::SaltString, Argon2, ParamsBuilder, PasswordHash, PasswordHasher};

fn argon2_with_salt<'a>(password: &'a str, salt: &'a SaltString) -> PasswordHash<'a> {
    let argon2 = Argon2::from({
        let mut builder = ParamsBuilder::default();
        builder.output_len(32).unwrap();
        builder.params().unwrap()
    });
    argon2.hash_password(password.as_bytes(), salt).unwrap()
}
