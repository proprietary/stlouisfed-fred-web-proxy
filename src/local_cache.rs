use chrono::NaiveDate;
use sqlite::{self, State};
use crate::entities::RealtimeObservation;

pub trait FREDDatabase {
    fn new(path: &std::path::Path) -> Self;
    fn open(&self) -> Result<sqlite::Connection, sqlite::Error>;
    fn create_tables(&self) -> Result<(), sqlite::Error>;
}

#[derive(Debug, Default, Clone)]
pub struct RealtimeObservationsDatabase {
    path: std::path::PathBuf,
}

impl FREDDatabase for RealtimeObservationsDatabase {
    fn new(path: &std::path::Path) -> RealtimeObservationsDatabase {
        RealtimeObservationsDatabase {
            path: path.to_path_buf(),
        }
    }

    fn open(&self) -> Result<sqlite::Connection, sqlite::Error> {
        let connection = sqlite::open(
            &self
                .path
                .to_str()
                .expect("Path for realtime observations sqlite file is bad"),
        )?;
        return Ok(connection);
    }

    fn create_tables(&self) -> Result<(), sqlite::Error> {
        let query = r#"
        create table if not exists realtime_observations (
            series_id text not null,
            date text not null check (date(`date`) > date('1776-07-04') and date(`date`) < date('9999-12-31')),
            value text not null,
            primary key (series_id, date)
        );
        "#;
        let conn = self.open()?;
        conn.execute(query)?;
        Ok(())
    }
}

impl RealtimeObservationsDatabase {
    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn get_observations(
        &self,
        series_id: &str,
        since: Option<NaiveDate>,
        until: Option<NaiveDate>,
    ) -> Result<Vec<RealtimeObservation>, sqlite::Error> {
        let conn = self.open()?;
        let query = r#"
        select `date`, `value`
        from realtime_observations
        where date(`date`) >= date(:since)
            and date(`date`) <= date(:until)
            and `series_id` = :series_id;
        "#;
        let mut statement = conn.prepare(query)?;
        let since_str = since
            .unwrap_or(NaiveDate::MIN)
            .format(Self::FORMAT)
            .to_string();
        let until_str = until
            .unwrap_or(NaiveDate::MAX)
            .format(Self::FORMAT)
            .to_string();
        statement.bind::<&[(_, sqlite::Value)]>(
            &[
                (":since", since_str.into()),
                (":until", until_str.into()),
                (":series_id", series_id.into()),
            ][..],
        )?;
        let mut dst = std::vec::Vec::<RealtimeObservation>::new();
        while let Ok(State::Row) = statement.next() {
            let value = statement.read::<String, _>("value")?;
            let date_str = statement.read::<String, _>("date")?;
            let date = NaiveDate::parse_from_str(&date_str, Self::FORMAT).map_err(|e| {
                let kind = e.kind();
                dbg!(
                    "Badly formatted date in database: {#:?} - error kind: {#:?}",
                    &date_str,
                    kind
                );
                sqlite::Error {
                    code: None,
                    message: Some("stored date formatted incorrectly".to_string()),
                }
            })?;
            dst.push(RealtimeObservation { date, value });
        }
        Ok(dst)
    }

    pub fn put_observations(
        &self,
        series_id: &str,
        rows: &[RealtimeObservation],
    ) -> Result<(), sqlite::Error> {
        let query = r#"
        insert into realtime_observations (`series_id`, `date`, `value`)
        values (:series_id, :date, :value)
        on conflict (`series_id`, `date`) do update set `value` = excluded.`value`;
        "#;
        let conn = self.open()?;
        let mut statement = conn.prepare(query)?;
        for row in rows.iter() {
            let date_str = row.date.format(Self::FORMAT).to_string();
            statement.reset()?;
            statement.bind::<&[(_, sqlite::Value)]>(
                &[
                    (":series_id", series_id.into()),
                    (":date", date_str.into()),
                    (":value", sqlite::Value::String(row.value.clone())),
                ][..],
            )?;
            statement.next()?;
        }
        Ok(())
    }
}

