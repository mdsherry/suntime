use std::collections::{HashMap, HashSet};

use flate2::read::GzDecoder;
use serde::Deserialize;
use suntime::Pos;
use thiserror::Error;

use crate::Args;

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Both lat or long must be provided, or neither")]
    BothOrNeitherLatLong,
    #[error("Only one of lat or long was supplied")]
    NoLocation,
    #[error("You must provide only one of a city name, or a lat/long coordinate")]
    AmbiguousLocation,
    #[error("Either latitude ({0}) or longitude ({1}) were out of range")]
    ValueOutOfRange(f64, f64),
    #[error("Unknown city {0}")]
    UnknownCity(String),
    #[error("Unknown format {0}")]
    UnknownFormat(String),
}

#[derive(Debug, Clone, Deserialize)]
struct LocationRow {
    city: String,
    city_ascii: String,
    lat: f64,
    lng: f64,
    country: String,
    iso2: String,
    iso3: String,
    admin_name: String,
}

impl LocationRow {
    fn to_pos(&self) -> Pos {
        Pos::new(self.lat, self.lng)
    }
}

fn load_loc_data() -> Vec<LocationRow> {
    let raw = &include_bytes!("worldcities.csv.gz")[..];
    let decoded = GzDecoder::new(raw);
    let mut rv = vec![];
    for row in csv::Reader::from_reader(decoded).deserialize() {
        let row: LocationRow = row.unwrap();
        rv.push(row);
    }
    rv
}

fn check_countries(name: &str, row: &LocationRow) -> bool {
    check_country(name, &row.country)
    || check_country(name, &row.iso2)
    || check_country(name, &row.iso3)
}

fn check_country(name: &str, country: &str) -> bool {
    let country = country.to_lowercase();
    name == country
}

fn check_state(name: &str, state: &str, row: &LocationRow) -> bool {
    let state = state.to_lowercase();
    if let Some(rest) = name.strip_prefix(&state) {
        if rest.trim().is_empty() {
            true
        } else if rest.starts_with(',') || rest.starts_with(' ') {
            let rest = rest.trim_start_matches(',').trim();
            check_countries(rest, row)
        } else {
            false
        }
    } else {
        false
    }
}

fn check_city(name: &str, city: &str, row: &LocationRow) -> bool {
    let city_low = city.to_lowercase();

    if let Some(rest) = name.strip_prefix(&city_low) {
        if rest.trim().is_empty() {
            true
        } else if rest.starts_with(',') || rest.starts_with(' ') {
            let rest = rest.trim_start_matches(',').trim();
            check_countries(rest, row)
                || (!row.admin_name.is_empty() && check_country(rest, &row.admin_name))
                || check_state(rest, &row.admin_name, row)
        } else {
            false
        }
    } else {
        false
    }
}

fn match_to_city<'a>(name: &str, locations: &'a [LocationRow]) -> Vec<&'a LocationRow> {
    locations
        .iter()
        .filter(|row| check_city(name, &row.city, row) || check_city(name, &row.city_ascii, row))
        .collect()
}

fn city_to_pos(city: &str) -> Result<Pos, LocationError> {
    let locations = load_loc_data();
    let city_low = city.to_lowercase();
    let city_results = match_to_city(&city_low, &locations);

    let suggestions = if city_results.len() == 1 {
        return Ok(city_results[0].to_pos());
    } else {
        city_results
    };

    if !suggestions.is_empty() {
        eprintln!("Multiple cities matched '{city}'. Did you mean:");

        let mut country_count: HashMap<&str, u32> = HashMap::new();
        for suggestion in &suggestions {
            *country_count.entry(&suggestion.iso2).or_default() += 1;
        }
        for suggestion in &suggestions {
            if country_count[&suggestion.iso2.as_str()] > 1 {
                eprintln!(
                    "  * {}, {}, {}",
                    suggestion.city, suggestion.admin_name, suggestion.iso2
                )
            } else {
                eprintln!("  * {}, {}", suggestion.city, suggestion.iso2)
            }
        }
    }
    Err(LocationError::UnknownCity(city.to_owned()))
}

pub fn validate_location(args: &Args) -> Result<Pos, LocationError> {
    let (lat, long, city) = if args.lat.is_none() && args.long.is_none() && args.city.is_none() {
        // Get values from env vars
        (
            env_arg_to_f64("SUNTIME_LAT"),
            env_arg_to_f64("SUNTIME_LONG"),
            std::env::var("SUNTIME_CITY").ok(),
        )
    } else {
        (args.lat, args.long, args.city.clone())
    };
    match (lat, long, &city) {
        (None, None, None) => Err(LocationError::NoLocation),
        (None, None, Some(city)) => city_to_pos(city),
        (None, Some(_), None) => Err(LocationError::BothOrNeitherLatLong),
        (None, Some(_), Some(city)) => city_to_pos(city),
        (Some(_), None, None) => Err(LocationError::BothOrNeitherLatLong),
        (Some(_), None, Some(city)) => city_to_pos(city),
        (Some(lat), Some(long), None)
            if (-90. ..=90.).contains(&lat) && (-180. ..=180.).contains(&long) =>
        {
            Ok(Pos::new(lat, long))
        }
        (Some(lat), Some(long), None) => Err(LocationError::ValueOutOfRange(lat, long)),
        (Some(_), Some(_), Some(_)) => Err(LocationError::AmbiguousLocation),
    }
}

fn env_arg_to_f64(name: &str) -> Option<f64> {
    std::env::var(name).ok().and_then(|s| {
        s.parse::<f64>()
            .map_err(|_| {
                eprintln!(
                    "Unable to parse environment variable {} as a float: {}",
                    name, s
                )
            })
            .ok()
    })
}
