const YYYY_MM_DD_FORMAT: &'static str = "%Y-%m-%d";

pub mod yyyy_mm_dd {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(super::YYYY_MM_DD_FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, super::YYYY_MM_DD_FORMAT).map_err(serde::de::Error::custom)
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
            let s = &date.format(super::YYYY_MM_DD_FORMAT).to_string();
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
                match NaiveDate::parse_from_str(&str, super::YYYY_MM_DD_FORMAT) {
                    Ok(x) => Ok(Some(x)),
                    Err(e) => Err(serde::de::Error::custom(e)),
                }
            }
        }
    }
}

pub mod iso_timestamp_string {
    use chrono::{DateTime, FixedOffset, Utc};
    use core::fmt;
    use serde::{de, ser};

    // ex: "2013-07-31 09:26:16-05"
    const FORMAT_SER: &'static str = "%Y-%m-%d %H:%M:%S%:::z";
    const FORMAT_DE: &'static str = "%Y-%m-%d %H:%M:%S%#z"; // hack; workaround some bug in chrono

    /// Serialize a UTC `DateTime` to a timestamp string in the format of "2013-07-31 09:26:16-05".
    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&dt.format(FORMAT_SER).to_string())
    }

    /// Deserialize timestamp string such as "2013-07-31 09:26:16-05" into a UTC `DateTime`.
    pub fn deserialize<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        d.deserialize_str(ISOTimestampStringVisitor)
    }

    struct ISOTimestampStringVisitor;

    impl<'de> de::Visitor<'de> for ISOTimestampStringVisitor {
        type Value = DateTime<Utc>;

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            DateTime::<FixedOffset>::parse_from_str(value, FORMAT_DE)
                .map(|x| x.with_timezone(&Utc))
                .map_err(E::custom)
        }

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter
                .write_str("a string datetime timestamp that looks like \"2013-07-31 09:26:16-05\"")
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_format_string() {
            let sample = "2023-09-18 19:10:56-05";
            let o = DateTime::<FixedOffset>::parse_from_str(sample, FORMAT_DE);
            assert!(o.is_ok());
            assert_eq!(o.unwrap().format(FORMAT_SER).to_string(), sample);
        }
    }
}
