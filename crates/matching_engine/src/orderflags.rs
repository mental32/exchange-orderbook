#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderFlags {
    pub post_only: bool,
    pub fee_in_base_currency: bool,
    pub fee_in_quote_currency: bool,
    pub volume_in_quote_currency: bool,
}

impl Default for OrderFlags {
    fn default() -> Self {
        Self {
            post_only: false,
            fee_in_base_currency: false,
            fee_in_quote_currency: false,
            volume_in_quote_currency: false,
        }
    }
}

pub fn serialize<S>(value: &OrderFlags, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    use serde::Serializer;
    let mut parts = vec![];
    if value.post_only {
        parts.push("post");
    }
    if value.fee_in_base_currency {
        parts.push("fcib");
    }
    if value.fee_in_quote_currency {
        parts.push("fciq");
    }
    if value.volume_in_quote_currency {
        parts.push("viqc");
    }
    parts.join(",").serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<OrderFlags, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    use serde::Deserializer;

    let st = String::deserialize(deserializer)?;

    let mut flags = OrderFlags::default();

    for part in st.split(',').map(str::trim) {
        match part {
            "post" => flags.post_only = true,
            "fcib" => {
                if flags.fee_in_quote_currency {
                    return Err(serde::de::Error::custom(
                        "fee_in_quote_currency and fee_in_base_currency cannot both be true",
                    ));
                } else {
                    flags.fee_in_base_currency = true
                }
            }
            "fciq" => {
                if flags.fee_in_base_currency {
                    return Err(serde::de::Error::custom(
                        "fee_in_quote_currency and fee_in_base_currency cannot both be true",
                    ));
                } else {
                    flags.fee_in_quote_currency = true
                }
            }
            "viqc" => flags.volume_in_quote_currency = true,
            "nompp" => (), // deprecated, ignored.
            _ => return Err(serde::de::Error::custom(format!("unknown flag: {}", part))),
        }
    }

    Ok(flags)
}

pub fn deserialize_orderflags_option<'de, D>(
    deserializer: D,
) -> Result<Option<OrderFlags>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OrderFlagsVisitor;

    impl<'de> serde::de::Visitor<'de> for OrderFlagsVisitor {
        type Value = Option<OrderFlags>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an order flags string or null")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let st = value;
            let mut flags = OrderFlags::default();

            for part in st.split(',').map(str::trim) {
                match part {
                    "post" => flags.post_only = true,
                    "fcib" => {
                        if flags.fee_in_quote_currency {
                            return Err(E::custom(
                                "fee_in_quote_currency and fee_in_base_currency cannot both be true",
                            ));
                        } else {
                            flags.fee_in_base_currency = true
                        }
                    }
                    "fciq" => {
                        if flags.fee_in_base_currency {
                            return Err(E::custom(
                                "fee_in_quote_currency and fee_in_base_currency cannot both be true",
                            ));
                        } else {
                            flags.fee_in_quote_currency = true
                        }
                    }
                    "viqc" => flags.volume_in_quote_currency = true,
                    "nompp" => (), // deprecated, ignored.
                    _ => return Err(E::custom(format!("unknown flag: {}", part))),
                }
            }

            Ok(Some(flags))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(OrderFlagsVisitor)
}

#[cfg(feature = "serde")]
pub fn serialize_orderflags_option<S>(
    value: &Option<OrderFlags>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(flags) => crate::orderflags::serialize(flags, serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod test {
    use crate::orderflags::OrderFlags;

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    struct Shim {
        #[cfg_attr(feature = "serde", serde(with = "crate::orderflags"))]
        flags: OrderFlags,
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_de_oflags_from_str() {
        let json = r#"{"flags": "post,fcib,viqc"}"#;
        let Shim { flags } = serde_json::from_str(json).unwrap();
        assert_eq!(
            flags,
            OrderFlags {
                post_only: true,
                fee_in_base_currency: true,
                fee_in_quote_currency: false,
                volume_in_quote_currency: true,
            }
        );
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_se_oflags() {
        let oflags = OrderFlags {
            post_only: true,
            fee_in_base_currency: true,
            fee_in_quote_currency: false,
            volume_in_quote_currency: true,
        };
        let json = serde_json::to_string(&Shim { flags: oflags }).unwrap();
        assert_eq!(json, r#"{"flags":"post,fcib,viqc"}"#);
    }
}
