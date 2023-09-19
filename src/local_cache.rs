use crate::entities::{FredEconomicDataSeries, RealtimeObservation};
use chrono::NaiveDate;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

#[derive(Debug, Clone)]
pub struct RealtimeObservationsDatabase {
    pool: Box<SqlitePool>,
}

impl RealtimeObservationsDatabase {
    pub async fn new(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let pathbuf = path.to_path_buf();
        let co: SqliteConnectOptions = SqliteConnectOptions::new()
            .filename(&pathbuf)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(co)
            .await?;
        Ok(RealtimeObservationsDatabase {
            pool: Box::new(pool),
        })
    }

    pub async fn create_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        let query = r#"
        create table if not exists realtime_observations (
            series_id text not null,
            date text not null check (date(`date`) > date('1776-07-04') and date(`date`) < date('9999-12-31')),
            value text not null,
            primary key (series_id, date)
        );

        create table if not exists economic_data_series (
            id text not null primary key,
            last_updated timestamp not null,
            observation_start date not null,
            observation_end date not null
        );
        "#;
        let mut conn = self.pool.clone().acquire().await?;
        sqlx::query(query).execute(&mut *conn).await?;
        Ok(())
    }

    pub async fn get_observations(
        &self,
        series_id: &str,
        since: Option<NaiveDate>,
        until: Option<NaiveDate>,
    ) -> Result<Vec<RealtimeObservation>, Box<dyn std::error::Error>> {
        let query = sqlx::query_as::<_, RealtimeObservation>(
            r#"
        select `date`, `value`
        from realtime_observations
        where date(`date`) >= date(?)
            and date(`date`) <= date(?)
            and `series_id` = ?
        "#,
        );
        let stream = query
            .bind(since.unwrap_or(NaiveDate::MIN))
            .bind(until.unwrap_or(NaiveDate::MAX))
            .bind(&series_id.to_string())
            .fetch_all(&(*self.pool).clone())
            .await?;
        Ok(stream)
    }

    pub async fn put_observations(
        &self,
        series_id: &str,
        rows: &[RealtimeObservation],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = (*self.pool).acquire().await?;
        for row in rows.iter() {
            let _ = sqlx::query(
                r#"
            insert into realtime_observations (`series_id`, `date`, `value`)
            values (?, ?, ?)
            on conflict (`series_id`, `date`) do update set `value` = excluded.`value`;
            "#,
            )
            .bind(&series_id.to_string())
            .bind(row.date.clone())
            .bind(row.value.clone())
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    pub async fn put_series(
        &self,
        series: &FredEconomicDataSeries,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query(
            r#"
        insert into economic_data_series (id, last_updated, observation_start, observation_end)
        values (?, ?, ?, ?)
        "#,
        )
        .bind(&series.id)
        .bind(&series.last_updated)
        .bind(series.observation_start)
        .bind(series.observation_end)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub async fn get_series(
        &self,
        series_id: &str,
    ) -> Result<Option<FredEconomicDataSeries>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.acquire().await?;
        let res: Option<FredEconomicDataSeries> = sqlx::query_as::<_, FredEconomicDataSeries>(
            r#"
        select id, last_updated, observation_start, observation_end
        from economic_data_series
        where id = ?;
        "#,
        )
        .bind(&series_id)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(res)
    }
}
