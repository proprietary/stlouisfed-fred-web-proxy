use chrono::NaiveDate;
use serde::{self, Serialize, Deserialize};
use crate::yyyy_mm_dd_date_format;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RealtimeObservation {
    #[serde(with = "yyyy_mm_dd_date_format")]
    pub date: NaiveDate,
    pub value: String,
}
