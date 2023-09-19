use crate::date_formats::{iso_timestamp_string, optional_date, yyyy_mm_dd};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct RealtimeObservation {
    #[serde(with = "yyyy_mm_dd")]
    pub date: NaiveDate,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GetSeriesParams {
    pub series_id: String,
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FredResponseError {
    pub error_message: String,
    pub error_code: u16,
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FredObservationsResponseWithError {
    Payload(FredResponseObservation),
    ErrorMessage(FredResponseError),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FredApiResponse<T> {
    Payload(T),
    ErrorMessage(FredResponseError),
}

/// Response JSON type from FRED API `/fred/series`
/// See: https://fred.stlouisfed.org/docs/api/fred/series.html
#[derive(Default, Debug, Deserialize)]
pub struct FredResponseSeries {
    #[serde(with = "yyyy_mm_dd")]
    pub realtime_start: NaiveDate,
    #[serde(with = "yyyy_mm_dd")]
    pub realtime_end: NaiveDate,
    pub seriess: Vec<FredEconomicDataSeries>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FredResponseSeriesWithError {
    FredResponseSeries(FredResponseSeries),
    FredResponseError(FredResponseError),
}

#[derive(Default, Debug, Deserialize, Serialize, Clone, sqlx::FromRow)]
pub struct FredEconomicDataSeries {
    pub id: String,
    #[serde(with = "iso_timestamp_string")]
    pub last_updated: DateTime<Utc>,
    #[serde(with = "yyyy_mm_dd")]
    pub realtime_start: NaiveDate,
    #[serde(with = "yyyy_mm_dd")]
    pub realtime_end: NaiveDate,
    pub title: String,
    #[serde(with = "yyyy_mm_dd")]
    pub observation_start: NaiveDate,
    #[serde(with = "yyyy_mm_dd")]
    pub observation_end: NaiveDate,
    pub frequency: String,
    pub frequency_short: String,
    pub units: String,
    pub units_short: String,
    pub seasonal_adjustment: String,
    pub seasonal_adjustment_short: String,
    pub popularity: i64,
    pub notes: String,
}

#[cfg(test)]
mod test {
    use super::{FredEconomicDataSeries, FredResponseSeries};

    #[test]
    fn test_decode_series_api_result() {
        // % curl "https://api.stlouisfed.org/fred/series?file_type=json&api_key=$FRED_API_KEY&series_id=SP500"
        let api_result = r#"{"realtime_start":"2023-09-19","realtime_end":"2023-09-19","seriess":[{"id":"SP500","realtime_start":"2023-09-19","realtime_end":"2023-09-19","title":"S&P 500","observation_start":"2013-09-19","observation_end":"2023-09-18","frequency":"Daily, Close","frequency_short":"D","units":"Index","units_short":"Index","seasonal_adjustment":"Not Seasonally Adjusted","seasonal_adjustment_short":"NSA","last_updated":"2023-09-18 19:10:56-05","popularity":82,"notes":"The observations for the S&P 500 represent the daily index value at market close. The market typically closes at 4 PM ET, except for holidays when it sometimes closes early.\r\n\r\nThe Federal Reserve Bank of St. Louis and S&P Dow Jones Indices LLC have reached a new agreement on the use of Standard & Poors and Dow Jones Averages series in FRED. FRED and its associated services will include 10 years of daily history for Standard & Poors and Dow Jones Averages series.\r\n\r\nThe S&P 500 is regarded as a gauge of the large cap U.S. equities market. The index includes 500 leading companies in leading industries of the U.S. economy, which are publicly held on either the NYSE or NASDAQ, and covers 75% of U.S. equities. Since this is a price index and not a total return index, the S&P 500 index here does not contain dividends.\r\n\r\nCopyright \u00a9 2016, S&P Dow Jones Indices LLC. All rights reserved. Reproduction of S&P 500 in any form is prohibited except with the prior written permission of S&P Dow Jones Indices LLC (\"S&P\"). S&P does not guarantee the accuracy, adequacy, completeness or availability of any information and is not responsible for any errors or omissions, regardless of the cause or for the results obtained from the use of such information. S&P DISCLAIMS ANY AND ALL EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, ANY WARRANTIES OF MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE OR USE. In no event shall S&P be liable for any direct, indirect, special or consequential damages, costs, expenses, legal fees, or losses (including lost income or lost profit and opportunity costs) in connection with subscriber's or others' use of S&P 500.\r\n\r\nPermission to reproduce S&P 500 can be requested from index_services@spdji.com. More contact details are available here (http:\/\/us.spindices.com\/contact-us), including phone numbers for all regional offices."}]}"#;
        assert!(serde_json::from_str::<FredResponseSeries>(api_result).is_ok());
        assert_eq!(
            serde_json::from_str::<FredResponseSeries>(api_result)
                .unwrap()
                .seriess
                .get(0)
                .unwrap()
                .id,
            "SP500"
        );
        let result = serde_json::from_str::<FredResponseSeries>(api_result).unwrap();
        let economic_data_series: &FredEconomicDataSeries = result.seriess.get(0).unwrap();
        assert_eq!(economic_data_series.title, "S&P 500");
    }
}
