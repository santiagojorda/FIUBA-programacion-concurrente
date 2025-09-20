use serde::Deserialize;
use std::net::Ipv4Addr;

pub(crate) fn deserialize_ip<'de, D>(deserializer: D) -> Result<Ipv4Addr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
