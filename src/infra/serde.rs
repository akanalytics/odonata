use serde::{Deserialize, Deserializer};


pub fn deserialize_bool_flexibly<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FlexibleBool {
        Int(i64),
        Boolean(bool),
    }

    match FlexibleBool::deserialize(deserializer)? {
        FlexibleBool::Boolean(b) => Ok(b),
        FlexibleBool::Int(i) => match i {
            0|1 => Ok(i != 0),
            _ => Err(serde::de::Error::custom("Only 0 or 1 can represent booleans")),
        }
    }
}
     