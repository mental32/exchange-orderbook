#[cfg(feature = "argon2")]
use {argon2::PasswordHasher, argon2::password_hash::PasswordHashString};

use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Default, Clone)]
pub struct Password(pub String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Password").finish()
    }
}

pub fn de_password_from_str<'de, D>(d: D) -> Result<Password, D::Error>
where
    D: Deserializer<'de>,
{
    struct St;

    impl<'de> Visitor<'de> for St {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a string")
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.to_owned())
        }
    }

    d.deserialize_string(St).map(Password)
}

impl Password {
    #[cfg(feature = "argon2")]
    pub fn argon2_hash_password(&self) -> Result<PasswordHashString, argon2::password_hash::Error> {
        let argon2 = argon2::Argon2::default();
        let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
        let password_hash = argon2.hash_password(self.0.as_bytes(), &salt)?;
        Ok(password_hash.serialize())
    }
}
