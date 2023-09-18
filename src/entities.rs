use crate::date_formats::{optional_date, yyyy_mm_dd};
use chrono::NaiveDate;
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct RealtimeObservation {
    #[serde(with = "yyyy_mm_dd")]
    pub date: NaiveDate,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GetObservationsParams {
    pub series_id: String,

    #[serde(default, with = "optional_date")]
    pub observation_start: Option<NaiveDate>,

    #[serde(default, with = "optional_date")]
    pub observation_end: Option<NaiveDate>,

    #[serde(default, with = "optional_date")]
    pub realtime_start: Option<NaiveDate>,

    #[serde(default, with = "optional_date")]
    pub realtime_end: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationItem {
    #[serde(with = "yyyy_mm_dd")]
    pub date: NaiveDate,

    pub value: String,

    #[serde(with = "yyyy_mm_dd")]
    pub realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd")]
    pub realtime_end: NaiveDate,
}

#[derive(Default, Debug, Deserialize)]
pub struct FredResponseObservation {
    #[serde(with = "yyyy_mm_dd")]
    #[allow(dead_code)]
    pub realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd")]
    #[allow(dead_code)]
    pub realtime_end: NaiveDate,

    #[allow(dead_code)]
    pub count: usize,

    #[allow(dead_code)]
    pub offset: usize,

    pub limit: usize,

    pub observations: std::vec::Vec<ObservationItem>,
}
