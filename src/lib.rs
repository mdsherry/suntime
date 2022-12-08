use chrono::{Duration, prelude::*};
use std::f64::consts::{TAU};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct FractionalYear(f64);
impl FractionalYear {
  pub fn sin(self) -> f64 { self.0.sin() }
  pub fn two_sin(self) -> f64 { (self.0 * 2.).sin() }
  pub fn three_sin(self) -> f64 { (self.0 * 3.).sin() }

  pub fn cos(self) -> f64 { self.0.cos() }
  pub fn two_cos(self) -> f64 { (self.0 * 2.).cos() }
  pub fn three_cos(self) -> f64 { (self.0 * 2.).cos() }
}

fn gamma(dt: DateTime<Utc>) -> FractionalYear {
  let day = (dt.ordinal() - 1) as f64 + (dt.hour() as f64 - 12.) / 24.;
  FractionalYear(day / 365. * TAU)
}



fn fract_minutes_to_dt(mut dt: Date<Utc>, minutes: f64) -> DateTime<Utc> {
  let mut h = (minutes / 60.) as u32;
  let m = minutes as u32 % 60;
  let s = (minutes.fract() * 60.) as u32;
  if h >= 24 {
      h -= 24;
      dt = dt + Duration::days(1);
  }
  dt.and_hms_opt(h, m, s).unwrap_or_else(|| {
      dbg!(h, m, s, dt);
      panic!("Invalid date?");
  })
}

#[derive(Debug, Clone, Copy)]
pub struct Pos {
    lat: f64,
    long: f64
}

impl Pos {
    pub fn new(lat: f64, long: f64) -> Self {
        Pos { lat, long }
    }

    fn _solar_noon(self, date: Date<Utc>, dt: DateTime<Utc>) -> DateTime<Utc> {
        let gamma = gamma(dt);
        let minutes = 720. - 4. * self.long - eqtime(gamma);
        fract_minutes_to_dt(date, minutes)
    }

    pub fn solar_noon(self, dt: DateTime<Utc>) -> DateTime<Utc> {
        self._solar_noon(dt.date(), self._solar_noon(dt.date(), dt))
    }

    fn _sunrise(self, date: Date<Utc>, dt: DateTime<Utc>) -> DateTime<Utc> {
        let gamma = gamma(dt);
        let ha = self.zenith_hour_angle(gamma);
        let minutes = 720. - 4. * (self.long + ha) - eqtime(gamma);
        fract_minutes_to_dt(date, minutes)
    }
    pub fn sunrise(self, dt: DateTime<Utc>) -> DateTime<Utc> {
        self._sunrise(dt.date(), self._sunrise(dt.date(), dt))
    }

    fn _sunset(self, date: Date<Utc>, dt: DateTime<Utc>) -> DateTime<Utc> {
        let gamma = gamma(dt);
        let ha = self.zenith_hour_angle(gamma);
        let minutes = 720. - 4. * (self.long - ha) - eqtime(gamma);
        fract_minutes_to_dt(date, minutes)
    }

    pub fn sunset(self, dt: DateTime<Utc>) -> DateTime<Utc> {
        self._sunset(dt.date(), self._sunset(dt.date(), dt))
    }

    fn zenith_hour_angle(self, gamma: FractionalYear) -> f64 {
        let decl = decl(gamma);
        let a = 90.883f64.to_radians().cos() / (self.lat.to_radians().cos() * decl.cos());
        let b = self.lat.to_radians().tan() * decl.tan();
        (a - b).acos().to_degrees()
    }
}

/// Equation of time
/// Returns the amount that actual solar time differs from ideal solar time at a given point in the year:
/// https://en.wikipedia.org/wiki/Equation_of_time
fn eqtime(gamma: FractionalYear) -> f64 {
  229.18 * (0.000_075
      + 0.001_868 * gamma.cos() - 0.032_077 * gamma.sin()
      - 0.014_615 * gamma.two_cos() - 0.040_849 * gamma.two_sin())
}

/// Returns the solar declention angle for a given fractional year
fn decl(gamma: FractionalYear) -> f64 {
  let decl = 0.006_918
      - 0.399_912 * gamma.cos() + 0.070_257 * gamma.sin()
      - 0.006_758 * gamma.two_cos() + 0.000_907 * gamma.two_sin()
      - 0.002_697 * gamma.three_cos() + 0.001_480 * gamma.three_sin();
  decl
}
