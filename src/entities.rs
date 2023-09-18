use crate::yyyy_mm_dd_date_format;
use chrono::NaiveDate;
use serde::{self, de, Deserialize, Deserializer, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct RealtimeObservation {
    #[serde(with = "yyyy_mm_dd_date_format")]
    pub date: NaiveDate,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GetObservationsParams {
    pub series_id: String,

    #[serde(default, with = "optional_date_serde")]
    pub observation_start: Option<NaiveDate>,

    #[serde(default, with = "optional_date_serde")]
    pub observation_end: Option<NaiveDate>,

    #[serde(default, with = "optional_date_serde")]
    pub realtime_start: Option<NaiveDate>,

    #[serde(default, with = "optional_date_serde")]
    pub realtime_end: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationItem {
    #[serde(with = "yyyy_mm_dd_date_format")]
    pub date: NaiveDate,

    pub value: String,

    #[serde(with = "yyyy_mm_dd_date_format")]
    pub realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd_date_format")]
    pub realtime_end: NaiveDate,
}

#[derive(Default, Debug, Deserialize)]
pub struct FredResponseObservation {
    #[serde(with = "yyyy_mm_dd_date_format")]
    #[allow(dead_code)]
    pub realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd_date_format")]
    #[allow(dead_code)]
    pub realtime_end: NaiveDate,

    #[allow(dead_code)]
    pub count: usize,

    #[allow(dead_code)]
    pub offset: usize,

    pub limit: usize,

    pub observations: std::vec::Vec<ObservationItem>,
}

/// Serde deserialization decorator to map empty Strings to None,
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: core::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => std::str::FromStr::from_str(s)
            .map_err(de::Error::custom)
            .map(Some),
    }
}

mod optional_date_serde {
    use chrono::NaiveDate;
    use serde::{self, de, Deserialize, Deserializer, Serializer};
    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn serialize<S>(date_: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref date) = *date_ {
            let s = &date.format(FORMAT).to_string();
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
                match NaiveDate::parse_from_str(&str, FORMAT) {
                    Ok(x) => Ok(Some(x)),
                    Err(e) => Err(serde::de::Error::custom(e)),
                }
            }
        }
    }
}
