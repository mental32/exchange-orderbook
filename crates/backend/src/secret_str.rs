#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SecretStr(pub String);

impl std::fmt::Debug for SecretStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SecretStr").finish()
    }
}

#[cfg(feature = "serde")]
pub fn de_password_from_str<'de, D>(d: D) -> Result<SecretStr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct St;

    impl<'de> serde::de::Visitor<'de> for St {
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

    d.deserialize_string(St).map(SecretStr)
}
