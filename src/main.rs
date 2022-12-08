use std::{error::Error, str::FromStr};

use chrono::{prelude::*, Duration};

use location::{LocationError, validate_location};
use clap::Parser;
use suntime::Pos;

mod location;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Csv
}

impl FromStr for Format {
    type Err = LocationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(Format::Human),
            "csv" => Ok(Format::Csv),
            _ => Err(LocationError::UnknownFormat(s.to_string()))
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
    #[command(subcommand)]
    mode: Option<Mode>,
    /// Output format: human or json
    #[arg(short, long, default_value="human")]
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
    Next {
        days: u16
    },
    /// Shows times for the previous given number of days
    Last {
        days: u16
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let pos = validate_location(&args)?;
    let mode = args.mode.unwrap_or(Mode::Today);

    let today = Local::now().with_hour(12).unwrap().with_timezone(&Utc);
    match mode {
        Mode::Today => info_for_day(today, pos, args.format),
        Mode::Week => {
            let day_of_week = today.weekday().num_days_from_monday() as i64;
            from_date_til(today - Duration::days(day_of_week), today + Duration::days(6 - day_of_week), |dt| info_for_day(dt, pos, args.format))
        }
        Mode::Month => {
            let month_start = today.with_day(1).unwrap();
            let month_end = today.with_day(31)
                .or_else(|| today.with_day(30))
                .or_else(|| today.with_day(29))
                .or_else(|| today.with_day(28)).expect("Shortest month has 28 days");
            from_date_til(month_start, month_end, |dt| info_for_day(dt, pos, args.format))
        },
        Mode::Year => {
            let year_start = today.with_ordinal(1).unwrap();
            let year_end = today.with_ordinal(366).or_else(|| today.with_ordinal(365)).expect("At least 365 days per year");
            from_date_til(year_start, year_end, |dt| info_for_day(dt, pos, args.format))
        },
        Mode::Next { days } => {
            from_date_til(today, today + Duration::days(days as i64), |dt| info_for_day(dt, pos, args.format));
        },
        Mode::Last { days } => {
            from_date_til( today - Duration::days(days as i64 - 1), today, |dt| info_for_day(dt, pos, args.format));
        },
    }

    Ok(())
}

fn from_date_til(start: DateTime<Utc>, end: DateTime<Utc>, f: impl Fn(DateTime<Utc>)) {
    let mut dt = start;
    while dt <= end {
        f(dt);
        dt += Duration::days(1);
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

fn info_for_day(dt: DateTime<Utc>, pos: Pos, format: Format) {
    let tz = chrono::FixedOffset::west_opt(Local::now().offset().local_minus_utc()).expect("Offset obtained from Chrono won't be out-of-bounds");
    let dt = pos.solar_noon(dt);
    let noon = pos.solar_noon(dt).with_timezone(&tz);
    let sunrise = pos.sunrise(dt).with_timezone(&tz);
    let sunset = pos.sunset(dt).with_timezone(&tz);
    
    let tomorrow_sunset = pos.sunset(dt + Duration::days(1));
    let sunset_delta = (tomorrow_sunset - sunset.with_timezone(&Utc)) - Duration::days(1);
    let tomorrow_sunrise = pos.sunrise(dt + Duration::days(1));
    let sunrise_delta = (tomorrow_sunrise - sunrise.with_timezone(&Utc)) - Duration::days(1);
    let day_length = sunset - sunrise;
    let tomorrow_day_length = tomorrow_sunset - tomorrow_sunrise;
    let day_length_delta = tomorrow_day_length - day_length;

    match format {
        Format::Human => {
            println!("{date} 🌅 {sunrise} (Δ{sunrise_delta:>5}) 🌞 {noon} ({day_length} Δ{day_length_delta:>5}) 🌇 {sunset} (Δ{sunset_delta:>5})",
                date=dt.format("%Y-%m-%d"),
                sunrise=sunrise.format("%H:%M:%S"),
                sunrise_delta=format_duration_ms(sunrise_delta),
                noon=noon.format("%H:%M:%S"),
                day_length=format_duration_hms(day_length),
                day_length_delta=format_duration_ms(day_length_delta),
                sunset=sunset.format("%H:%M:%S"),
                sunset_delta=format_duration_ms(sunset_delta)
            );
        },
        Format::Csv => {
            let day_start = sunrise.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
            // println!("{:?} {:?} {:?}", day_start, sunrise, sunset);
            println!("{date},{sunrise},{noon},{sunset},{day_length}",
                date=dt.format("%Y-%m-%d"),
                sunrise=(sunrise - day_start).num_seconds(),
                noon=(noon - day_start).num_seconds(),
                sunset=(sunset - day_start).num_seconds(),
                day_length=(sunset - sunrise).num_seconds()
            )
        },
    }
    
}
