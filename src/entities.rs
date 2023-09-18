use crate::yyyy_mm_dd_date_format;
use chrono::NaiveDate;
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RealtimeObservation {
    #[serde(with = "yyyy_mm_dd_date_format")]
    pub date: NaiveDate,
    pub value: String,
}
