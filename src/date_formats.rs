const FORMAT: &'static str = "%Y-%m-%d";

pub mod yyyy_mm_dd {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(super::FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, super::FORMAT).map_err(serde::de::Error::custom)
    }
}

pub mod optional_date {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date_: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref date) = *date_ {
            let s = &date.format(super::FORMAT).to_string();
            serializer.serialize_str(s)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            None => Ok(None),
            Some(str) => {
                if str.len() == 0 {
                    return Ok(None);
                }
                match NaiveDate::parse_from_str(&str, super::FORMAT) {
                    Ok(x) => Ok(Some(x)),
                    Err(e) => Err(serde::de::Error::custom(e)),
                }
            }
        }
    }
}
