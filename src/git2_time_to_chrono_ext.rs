use anyhow::*;
use chrono::TimeZone;

/// An extension trait to convert `git2::Time` to `chrono::DateTime`.
/// # Examples
/// ```no_run
/// use git_iblame::Git2TimeToChronoExt;
///
/// // Print `git2::Time` to `stdout`.
/// fn print_git2_time(time: git2::Time) {
///   println!("{}", time.to_local_date_time().unwrap());
/// }
///
/// // Convert `git2::Time` to `Stirng` in the specified format.
/// fn git2_time_to_string(time: git2::Time) -> String {
///   time.to_local_date_time().unwrap().format("%Y-%m-%d %H:%M").to_string()
/// }
/// ```
pub trait Git2TimeToChronoExt {
    /// Convert `git2::Time` to `chrono::DateTime<chrono::FixedOffset>`.
    /// The time zone offset is the value in the `git2::Time`.
    /// # Examples
    /// ```
    /// use git_iblame::Git2TimeToChronoExt;
    /// // The Eastern Hemisphere time zone.
    /// let east_time = git2::Time::new(1745693791, 540);
    /// let east_datetime = east_time.to_date_time();
    /// assert!(east_datetime.is_ok());
    /// assert_eq!(east_datetime.unwrap().to_string(), "2025-04-27 03:56:31 +09:00");
    /// ```
    /// ```
    /// use git_iblame::Git2TimeToChronoExt;
    /// // The Western Hemisphere time zone.
    /// let west_time = git2::Time::new(1745196130, -420);
    /// let west_datetime = west_time.to_date_time();
    /// assert!(west_datetime.is_ok());
    /// assert_eq!(west_datetime.unwrap().to_string(), "2025-04-20 17:42:10 -07:00");
    /// ```
    fn to_date_time(&self) -> anyhow::Result<chrono::DateTime<chrono::FixedOffset>>;

    /// Convert `git2::Time` to `chrono::DateTime` in the specified time zone.
    /// # Examples
    /// ```
    /// use git_iblame::Git2TimeToChronoExt;
    /// let time = git2::Time::new(1745196130, -420);
    /// let utc_datetime = time.to_date_time_in(&chrono::Utc);
    /// assert_eq!(utc_datetime.unwrap().to_string(), "2025-04-21 00:42:10 UTC");
    /// ```
    fn to_date_time_in<Tz: chrono::TimeZone>(
        &self,
        tz: &Tz,
    ) -> anyhow::Result<chrono::DateTime<Tz>>;

    /// Convert `git2::Time` to `chrono::DateTime` in the local time zone.
    /// This function is a shorthand of:
    /// ```
    /// # use git_iblame::Git2TimeToChronoExt;
    /// # let time = git2::Time::new(1745693791, 540);
    /// let local_datetime = time.to_date_time_in(&chrono::Local);
    /// ```
    fn to_local_date_time(&self) -> anyhow::Result<chrono::DateTime<chrono::Local>>;
}

impl Git2TimeToChronoExt for git2::Time {
    fn to_date_time(&self) -> anyhow::Result<chrono::DateTime<chrono::FixedOffset>> {
        let tz = chrono::FixedOffset::east_opt(self.offset_minutes() * 60);
        if tz.is_none() {
            bail!("Invalid TimeZone {}", self.offset_minutes());
        }
        match tz.unwrap().timestamp_opt(self.seconds(), 0) {
            chrono::MappedLocalTime::Single(datetime) => Ok(datetime),
            chrono::MappedLocalTime::Ambiguous(_, latest) => Ok(latest),
            chrono::MappedLocalTime::None => bail!(
                "Time {} isn't mappable to {}",
                self.seconds(),
                self.offset_minutes()
            ),
        }
    }

    fn to_date_time_in<Tz: chrono::TimeZone>(
        &self,
        tz: &Tz,
    ) -> anyhow::Result<chrono::DateTime<Tz>> {
        self.to_date_time()
            .map(|datetime| datetime.with_timezone(tz))
    }

    fn to_local_date_time(&self) -> anyhow::Result<chrono::DateTime<chrono::Local>> {
        self.to_date_time_in(&chrono::Local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_date_time_offset_invalid() {
        let time = git2::Time::new(0, 100_000);
        let datetime = time.to_date_time();
        assert!(datetime.is_err());
        assert_eq!(datetime.unwrap_err().to_string(), "Invalid TimeZone 100000");
    }
}
