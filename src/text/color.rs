use cairo::Context;
use std::{fmt::{self, Display}, error::Error, num::ParseIntError, str::FromStr};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color(f64, f64, f64, f64);

macro_rules! color {
    ($name:ident,($r:expr, $g:expr, $b:expr)) => {
        #[allow(dead_code)]
        pub fn $name() -> Color {
            Color(1.0, $r, $g, $b)
        }
    };
}

impl Color {
    color!(red, (1.0, 0.0, 0.0));
    color!(green, (0.0, 1.0, 0.0));
    color!(blue, (0.0, 0.0, 1.0));
    color!(white, (1.0, 1.0, 1.0));
    color!(black, (0.0, 0.0, 0.0));

    pub fn apply_to_context(&self, cr: &Context) {
        cr.set_source_rgb(self.0, self.1, self.2);
    }
}

impl FromStr for Color {
    type Err = ParseError;
    fn from_str(color: &str) -> Result<Color, Self::Err> {
        let color = color.trim_matches('#');
        match color.len() {
            0 => Ok(Color(1.0,0.0, 0.0, 0.0)),
            1..=2 => Ok(Color(1.0,0.0, 0.0, u8::from_str_radix(color, 16)? as f64 / 255.0)),
            3..=4 => {
                let (g, b) = color.split_at(color.len() - 2);
                Ok(Color(
                        1.0,
                    0.0,
                    u8::from_str_radix(g, 16)? as f64 / 255.0,
                    u8::from_str_radix(b, 16)? as f64 / 255.0,
                ))
            }
            5..=6 => {
                let (rest, b) = color.split_at(color.len() - 2);
                let (r, g) = rest.split_at(rest.len() - 2);
                Ok(Color(
                        1.0,
                    u8::from_str_radix(r, 16)? as f64 / 255.0,
                    u8::from_str_radix(g, 16)? as f64 / 255.0,
                    u8::from_str_radix(b, 16)? as f64 / 255.0,
                ))
            }
            7..=8 => {
                let (rest, a) = color.split_at(color.len() - 2);
                let (rest, b) = rest.split_at(rest.len() - 2);
                let (r, g) = rest.split_at(rest.len() - 2);
                Ok(Color(
                    u8::from_str_radix(a, 16)? as f64 / 255.0,
                    u8::from_str_radix(r, 16)? as f64 / 255.0,
                    u8::from_str_radix(g, 16)? as f64 / 255.0,
                    u8::from_str_radix(b, 16)? as f64 / 255.0,
                ))
            }
            v => Err(ParseError::Length(v)), //panic!("String '{}' too long: {}", color, v),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    ParseIntError(ParseIntError),
    Length(usize),
}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ParseIntError(p) => write!(f, "{}", p),
            Self::Length(l) => write!(f, "String too long: {}", l),
        }
    }
}

impl Error for ParseError {}
