use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

pub trait Clock: Send + Sync {
    /// Returns the current Unix timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the clock cannot represent a Unix timestamp.
    fn unix_seconds(&self) -> Result<i64>;
}

pub trait LocalTimezone: Send + Sync {
    /// Returns the local UTC offset at one Unix timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when local timezone data is unavailable or malformed.
    fn offset_at(&self, unix_seconds: i64) -> Result<i32>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_seconds(&self) -> Result<i64> {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::new("invalid_timestamp", "system time is before Unix epoch"))?
            .as_secs();
        i64::try_from(seconds)
            .map_err(|_| Error::new("invalid_timestamp", "system time is out of range"))
    }
}

#[derive(Debug, Default)]
pub struct SystemLocalTimezone;

impl LocalTimezone for SystemLocalTimezone {
    fn offset_at(&self, unix_seconds: i64) -> Result<i32> {
        let data = fs::read("/etc/localtime")
            .map_err(|error| Error::io("cannot read local timezone", &error))?;
        tzif_offset(&data, unix_seconds)
    }
}

pub(super) fn startup_timestamp(
    environment: &BTreeMap<OsString, OsString>,
    clock: &dyn Clock,
    timezone: &dyn LocalTimezone,
) -> Result<String> {
    if let Some(value) = environment.get(OsStr::new("WEYRIVA_STARTUP_TIMESTAMP")) {
        let value = value.to_string_lossy().into_owned();
        if valid_timestamp(&value) {
            return Ok(value);
        }
        return Err(Error::new(
            "invalid_timestamp",
            "invalid Weyriva startup backup timestamp",
        ));
    }
    let seconds = clock.unix_seconds()?;
    local_timestamp(seconds, timezone.offset_at(seconds)?)
}

fn valid_timestamp(value: &str) -> bool {
    value.len() == 15
        && value.as_bytes()[8] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 8 || byte.is_ascii_digit())
}

fn local_timestamp(seconds: i64, offset: i32) -> Result<String> {
    let local = seconds
        .checked_add(i64::from(offset))
        .ok_or_else(|| Error::new("invalid_timestamp", "local time is out of range"))?;
    let days = local.div_euclid(86_400);
    let day_seconds = local.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    Ok(format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        day_seconds / 3600,
        day_seconds / 60 % 60,
        day_seconds % 60
    ))
}

fn tzif_offset(data: &[u8], unix_seconds: i64) -> Result<i32> {
    let first = Header::read(data, 0)?;
    let (header, time_size) = if first.version == b'2' || first.version == b'3' {
        let second_offset = 44usize
            .checked_add(first.block_size(4)?)
            .ok_or_else(invalid_timezone)?;
        (Header::read(data, second_offset)?, 8)
    } else {
        (first, 4)
    };
    let block = header.offset + 44;
    let transitions_end = block
        .checked_add(
            header
                .timecnt
                .checked_mul(time_size)
                .ok_or_else(invalid_timezone)?,
        )
        .ok_or_else(invalid_timezone)?;
    let indexes_end = transitions_end
        .checked_add(header.timecnt)
        .ok_or_else(invalid_timezone)?;
    let types_end = indexes_end
        .checked_add(header.typecnt.checked_mul(6).ok_or_else(invalid_timezone)?)
        .ok_or_else(invalid_timezone)?;
    if types_end > data.len() || header.typecnt == 0 {
        return Err(invalid_timezone());
    }
    let mut selected = 0usize;
    for index in 0..header.timecnt {
        let start = block + index * time_size;
        let transition = read_signed(data, start, time_size)?;
        if transition > unix_seconds {
            break;
        }
        selected = usize::from(data[transitions_end + index]);
    }
    if selected >= header.typecnt {
        return Err(invalid_timezone());
    }
    read_i32(data, indexes_end + selected * 6)
}

#[derive(Clone, Copy)]
struct Header {
    offset: usize,
    version: u8,
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
    leapcnt: usize,
    ttisstdcnt: usize,
    ttisgmtcnt: usize,
}

impl Header {
    fn read(data: &[u8], offset: usize) -> Result<Self> {
        let end = offset.checked_add(44).ok_or_else(invalid_timezone)?;
        if data.get(offset..offset + 4) != Some(b"TZif") || end > data.len() {
            return Err(invalid_timezone());
        }
        Ok(Self {
            offset,
            version: data[offset + 4],
            ttisgmtcnt: read_count(data, offset + 20)?,
            ttisstdcnt: read_count(data, offset + 24)?,
            leapcnt: read_count(data, offset + 28)?,
            timecnt: read_count(data, offset + 32)?,
            typecnt: read_count(data, offset + 36)?,
            charcnt: read_count(data, offset + 40)?,
        })
    }

    fn block_size(self, time_size: usize) -> Result<usize> {
        self.timecnt
            .checked_mul(time_size)
            .and_then(|value| value.checked_add(self.timecnt))
            .and_then(|value| value.checked_add(self.typecnt.checked_mul(6)?))
            .and_then(|value| value.checked_add(self.charcnt))
            .and_then(|value| value.checked_add(self.leapcnt.checked_mul(time_size + 4)?))
            .and_then(|value| value.checked_add(self.ttisstdcnt))
            .and_then(|value| value.checked_add(self.ttisgmtcnt))
            .ok_or_else(invalid_timezone)
    }
}

fn read_count(data: &[u8], offset: usize) -> Result<usize> {
    usize::try_from(read_i32(data, offset)?).map_err(|_| invalid_timezone())
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(invalid_timezone)?
        .try_into()
        .map_err(|_| invalid_timezone())?;
    Ok(i32::from_be_bytes(bytes))
}

fn read_signed(data: &[u8], offset: usize, size: usize) -> Result<i64> {
    match size {
        4 => Ok(i64::from(read_i32(data, offset)?)),
        8 => {
            let bytes: [u8; 8] = data
                .get(offset..offset + 8)
                .ok_or_else(invalid_timezone)?
                .try_into()
                .map_err(|_| invalid_timezone())?;
            Ok(i64::from_be_bytes(bytes))
        }
        _ => Err(invalid_timezone()),
    }
}

fn invalid_timezone() -> Error {
    Error::new("invalid_timestamp", "local timezone data is malformed")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}
