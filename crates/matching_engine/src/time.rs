use std::str::FromStr;

use chumsky::text::digits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Time {
    Now,
    UnixTimestamp(u64),
    Scheduled(u64),
}

impl FromStr for Time {
    type Err = ();

    fn from_str(st: &str) -> Result<Self, Self::Err> {
        use chumsky::prelude::*;
        use chumsky::text::newline;

        let digits = text::digits::<_, extra::Err<EmptyErr>>(10).to_slice();

        just("0")
            .map(|_| Time::Now)
            .or(just("+")
                .ignore_then(digits)
                .try_map(|substr: &str, _| substr.parse().map_err(|_| EmptyErr::default()))
                .map(|n: u64| Time::Scheduled(n)))
            .parse(st)
            .into_output()
            .ok_or(())
    }
}

impl Default for Time {
    fn default() -> Self {
        Time::Now
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Time::Now => write!(f, "0"),
            Time::UnixTimestamp(ts) => write!(f, "{}", ts),
            Time::Scheduled(ts) => write!(f, "+{}", ts),
        }
    }
}

impl Time {
    pub fn to_absolute_timestamp(&self, now: u64) -> Option<u64> {
        match self {
            Time::Now => None,
            Time::UnixTimestamp(ts) => Some(*ts),
            Time::Scheduled(offset) => Some(now.saturating_add(*offset)),
        }
    }
}

#[cfg(feature = "serde")]
pub fn serialize<S>(value: &Time, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize as _;

    value.to_string().serialize(serializer)
}

#[cfg(feature = "serde")]
pub fn deserialize<'de, D>(deserializer: D) -> Result<Time, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let st: String = String::deserialize(deserializer)?;

    Time::from_str(&st).map_err(|_| serde::de::Error::custom("Invalid time format"))
}

#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::time::Time;

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    struct Shim {
        #[cfg_attr(feature = "serde", serde(with = "crate::time"))]
        time: Time,
    }

    #[test]
    fn test_time_from_str() {
        assert_eq!(Time::from_str("+27"), Ok(Time::Scheduled(27)));
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_de_time() {
        let json = r#"{"time": "+23"}"#;
        let shim: Shim = serde_json::from_str(json).unwrap();
        assert_eq!(shim.time, Time::Scheduled(23));
    }

    #[cfg_attr(feature = "serde", test)]
    fn test_ser_time() {
        let time = Shim {
            time: Time::Scheduled(23),
        };
        let json = serde_json::to_string(&time).unwrap();
        assert_eq!(json, r#"{"time":"+23"}"#);
    }
}
