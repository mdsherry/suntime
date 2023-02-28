use std::{error::Error, str::FromStr};

use chrono::{prelude::*, Duration};

use clap::Parser;
use location::{validate_location, LocationError};
use plot::plot_times;
use serde::{Serialize, Serializer};
use suntime::Pos;

mod location;
mod plot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Csv,
    Json,
    Plot,
}

impl FromStr for Format {
    type Err = LocationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(Format::Human),
            "csv" => Ok(Format::Csv),
            "json" => Ok(Format::Json),
            "plot" => Ok(Format::Plot),
            _ => Err(LocationError::UnknownFormat(s.to_string())),
        }
    }
}

#[derive(Parser)]
/// Sunrise/set table generator
///
/// Uses location data from https://simplemaps.com/data/world-cities
pub struct Args {
    #[arg(short, long)]
    /// Location name in the form "City", "City, Country (Code)", or "City, State, Country (Code)"
    city: Option<String>,
    #[arg(long)]
    /// Latitude; requires longitude as well, and is incompatible with --city
    lat: Option<f64>,
    #[arg(long)]
    /// Longitude; requires latitude as well, and is incompatible with --city
    long: Option<f64>,
    /// Plot width. Default: 120
    #[arg(long)]
    width: Option<usize>,
    /// Plot height. Default: 10
    #[arg(long)]
    height: Option<usize>,

    #[command(subcommand)]
    mode: Option<Mode>,
    /// Output format: human, csv, json or plot
    #[arg(short, long, default_value = "human")]
    format: Format,
}

#[derive(Parser)]
enum Mode {
    /// Shows times for today
    Today,
    /// Shows times for the current week
    Week,
    /// Shows times for the current month
    Month,
    /// Shows times for the current year
    Year,
    /// Shows times for the next given number of days
    Next { days: u16 },
    /// Shows times for the previous given number of days
    Last { days: u16 },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let pos = validate_location(&args)?;
    let mode = args.mode.unwrap_or(Mode::Today);

    let today = Local::now()
        .with_hour(12)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_timezone(&Utc);
    match mode {
        Mode::Today => output_range(
            DateIter::new(today, today),
            pos,
            args.width,
            args.height,
            args.format,
        ),
        Mode::Week => {
            let day_of_week = today.weekday().num_days_from_monday() as i64;
            output_range(
                DateIter::new(
                    today - Duration::days(day_of_week),
                    today + Duration::days(6 - day_of_week),
                ),
                pos,
                args.width,
                args.height,
                args.format,
            );
        }
        Mode::Month => {
            let month_start = today.with_day(1).unwrap();
            let month_end = today
                .with_day(31)
                .or_else(|| today.with_day(30))
                .or_else(|| today.with_day(29))
                .or_else(|| today.with_day(28))
                .expect("Shortest month has 28 days");

            output_range(
                DateIter::new(month_start, month_end),
                pos,
                args.width,
                args.height,
                args.format,
            )
        }
        Mode::Year => {
            let year_start = today.with_ordinal(1).unwrap();
            let year_end = today
                .with_ordinal(366)
                .or_else(|| today.with_ordinal(365))
                .expect("At least 365 days per year");
            output_range(
                DateIter::new(year_start, year_end),
                pos,
                args.width,
                args.height,
                args.format,
            )
        }
        Mode::Next { days } => output_range(
            DateIter::new(today, today + Duration::days(days as i64)),
            pos,
            args.width,
            args.height,
            args.format,
        ),
        Mode::Last { days } => output_range(
            DateIter::new(today - Duration::days(days as i64 - 1), today),
            pos,
            args.width,
            args.height,
            args.format,
        ),
    }

    Ok(())
}

fn output_range<I: Iterator<Item = DateTime<Utc>>>(
    range: I,
    pos: Pos,
    width: Option<usize>,
    height: Option<usize>,
    format: Format,
) {
    match format {
        Format::Human => range.for_each(|date| human_output(date, pos)),
        Format::Csv => range.for_each(|date| csv_output(date, pos)),
        Format::Plot => {
            let output: Vec<_> = range.map(|dt| SunTimes::from_pos(dt, pos)).collect();
            let sunsets: Vec<_> = output.iter().map(|s| s.sunset).collect();
            plot_times(
                "Sunsets",
                width.unwrap_or(120),
                height.unwrap_or(10),
                &sunsets,
            );
            let sunrises: Vec<_> = output.iter().map(|s| s.sunrise).collect();
            plot_times(
                "Sunrises",
                width.unwrap_or(120),
                height.unwrap_or(10),
                &sunrises,
            );
        }
        Format::Json => {
            let output: Vec<_> = range.map(|dt| SunTimes::from_pos(dt, pos)).collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }
}

struct DateIter {
    today: DateTime<Utc>,
    target: DateTime<Utc>,
}
impl Iterator for DateIter {
    type Item = DateTime<Utc>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.today <= self.target {
            let rv = self.today;
            self.today += Duration::days(1);
            Some(rv)
        } else {
            None
        }
    }
}

impl DateIter {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        DateIter {
            today: start,
            target: end,
        }
    }
}

fn format_duration_ms(duration: Duration) -> String {
    format!(
        "{}{}:{:02}",
        if duration.num_seconds() < 0 { "-" } else { "" },
        duration.num_minutes().abs(),
        (duration.num_seconds() % 60).abs()
    )
}

fn format_duration_hms(duration: Duration) -> String {
    format!(
        "{}{}:{:02}:{:02}",
        if duration.num_seconds() < 0 { "-" } else { "" },
        duration.num_hours(),
        duration.num_minutes().abs() % 60,
        (duration.num_seconds() % 60).abs()
    )
}

#[derive(Debug, Serialize)]
struct SunTimes {
    #[serde(serialize_with = "serialize_dt")]
    sunrise: DateTime<FixedOffset>,
    #[serde(serialize_with = "serialize_dt")]
    noon: DateTime<FixedOffset>,
    #[serde(serialize_with = "serialize_dt")]
    sunset: DateTime<FixedOffset>,
}

fn serialize_dt<S>(value: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.to_rfc3339().serialize(serializer)
}

impl SunTimes {
    fn from_pos(dt: DateTime<Utc>, pos: Pos) -> Self {
        let tz = chrono::FixedOffset::east_opt(Local::now().offset().local_minus_utc())
            .expect("Offset obtained from Chrono won't be out-of-bounds");
        let dt = pos.solar_noon(dt);
        let noon = pos.solar_noon(dt).with_timezone(&tz);
        let sunrise = pos.sunrise(dt).with_timezone(&tz);
        let sunset = pos.sunset(dt).with_timezone(&tz);
        SunTimes {
            sunrise,
            noon,
            sunset,
        }
    }
    fn day_length(&self) -> Duration {
        self.sunset - self.sunrise
    }
}

fn human_output(dt: DateTime<Utc>, pos: Pos) {
    let times = SunTimes::from_pos(dt, pos);
    let tomorrow = SunTimes::from_pos(dt + Duration::days(1), pos);

    let sunset_delta = (tomorrow.sunset - times.sunset) - Duration::days(1);
    let sunrise_delta = (tomorrow.sunrise - times.sunrise) - Duration::days(1);
    let day_length = times.day_length();
    let tomorrow_day_length = tomorrow.day_length();
    let day_length_delta = tomorrow_day_length - day_length;

    println!("{date} 🌅 {sunrise} (Δ{sunrise_delta:>5}) 🌞 {noon} ({day_length} Δ{day_length_delta:>5}) 🌇 {sunset} (Δ{sunset_delta:>5})",
        date=dt.format("%Y-%m-%d"),
        sunrise=times.sunrise.format("%H:%M:%S"),
        sunrise_delta=format_duration_ms(sunrise_delta),
        noon=times.noon.format("%H:%M:%S"),
        day_length=format_duration_hms(day_length),
        day_length_delta=format_duration_ms(day_length_delta),
        sunset=times.sunset.format("%H:%M:%S"),
        sunset_delta=format_duration_ms(sunset_delta)
    );
}

fn csv_output(dt: DateTime<Utc>, pos: Pos) {
    let times = SunTimes::from_pos(dt, pos);

    let day_start = times
        .sunrise
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    println!(
        "{date},{sunrise},{noon},{sunset},{day_length}",
        date = dt.format("%Y-%m-%d"),
        sunrise = (times.sunrise - day_start).num_seconds(),
        noon = (times.noon - day_start).num_seconds(),
        sunset = (times.sunset - day_start).num_seconds(),
        day_length = times.day_length().num_seconds()
    );
}

// fn info_for_day(dt: DateTime<Utc>, pos: Pos, format: Format) {
//     let tz = chrono::FixedOffset::west_opt(Local::now().offset().local_minus_utc()).expect("Offset obtained from Chrono won't be out-of-bounds");
//     let dt = pos.solar_noon(dt);
//     let noon = pos.solar_noon(dt).with_timezone(&tz);
//     let sunrise = pos.sunrise(dt).with_timezone(&tz);
//     let sunset = pos.sunset(dt).with_timezone(&tz);

//     let tomorrow_sunset = pos.sunset(dt + Duration::days(1));
//     let sunset_delta = (tomorrow_sunset - sunset.with_timezone(&Utc)) - Duration::days(1);
//     let tomorrow_sunrise = pos.sunrise(dt + Duration::days(1));
//     let sunrise_delta = (tomorrow_sunrise - sunrise.with_timezone(&Utc)) - Duration::days(1);
//     let day_length = sunset - sunrise;
//     let tomorrow_day_length = tomorrow_sunset - tomorrow_sunrise;
//     let day_length_delta = tomorrow_day_length - day_length;

//     match format {
//         Format::Human => {
//             println!("{date} 🌅 {sunrise} (Δ{sunrise_delta:>5}) 🌞 {noon} ({day_length} Δ{day_length_delta:>5}) 🌇 {sunset} (Δ{sunset_delta:>5})",
//                 date=dt.format("%Y-%m-%d"),
//                 sunrise=sunrise.format("%H:%M:%S"),
//                 sunrise_delta=format_duration_ms(sunrise_delta),
//                 noon=noon.format("%H:%M:%S"),
//                 day_length=format_duration_hms(day_length),
//                 day_length_delta=format_duration_ms(day_length_delta),
//                 sunset=sunset.format("%H:%M:%S"),
//                 sunset_delta=format_duration_ms(sunset_delta)
//             );
//         },
//         Format::Csv => {
//             let day_start = sunrise.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
//             // println!("{:?} {:?} {:?}", day_start, sunrise, sunset);
//             println!("{date},{sunrise},{noon},{sunset},{day_length}",
//                 date=dt.format("%Y-%m-%d"),
//                 sunrise=(sunrise - day_start).num_seconds(),
//                 noon=(noon - day_start).num_seconds(),
//                 sunset=(sunset - day_start).num_seconds(),
//                 day_length=(sunset - sunrise).num_seconds()
//             )
//         },
//         Format::Plot => {

//         }
//     }

// }
