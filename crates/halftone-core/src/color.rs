//! An 8-bit RGBA color with hex string serde support.

use std::fmt;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An 8-bit RGBA color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parse `#rrggbb`, `#rrggbbaa`, `rrggbb`, `rrggbbaa`, `none` or
    /// `transparent`. The leading `#` is optional.
    pub fn from_hex(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("transparent") {
            return Some(Self::TRANSPARENT);
        }
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 && s.len() != 8 {
            return None;
        }
        let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&s[range], 16).ok();
        let r = parse(0..2)?;
        let g = parse(2..4)?;
        let b = parse(4..6)?;
        let a = if s.len() == 8 { parse(6..8)? } else { 255 };
        Some(Self::new(r, g, b, a))
    }

    /// Lowercase hex string, with alpha only when it is not opaque.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// CSS `<color>` without the alpha component.
    pub fn to_rgb_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn alpha_f32(self) -> f32 {
        self.a as f32 / 255.0
    }

    pub fn is_transparent(self) -> bool {
        self.a == 0
    }

    /// Composite this color over an opaque white background, returning RGB.
    pub fn over_white(self) -> [u8; 3] {
        let a = self.alpha_f32();
        let mix = |v: u8| (v as f32 * a + 255.0 * (1.0 - a)).round() as u8;
        [mix(self.r), mix(self.g), mix(self.b)]
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl FromStr for Rgba {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_hex(s).ok_or_else(|| format!("invalid color '{s}', expected #rrggbb"))
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Rgba::from_str(&raw).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_and_eight_digit_hex() {
        assert_eq!(Rgba::from_hex("#ff0000"), Some(Rgba::rgb(255, 0, 0)));
        assert_eq!(Rgba::from_hex("00ff0080"), Some(Rgba::new(0, 255, 0, 128)));
    }

    #[test]
    fn parses_none_as_transparent() {
        assert!(Rgba::from_hex("none").unwrap().is_transparent());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Rgba::from_hex("#12").is_none());
        assert!(Rgba::from_hex("#gggggg").is_none());
    }

    #[test]
    fn round_trips_through_hex() {
        let c = Rgba::new(1, 2, 3, 4);
        assert_eq!(Rgba::from_hex(&c.to_hex()), Some(c));
    }
}
