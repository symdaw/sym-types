use derive_more::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Add,
    Sub,
    Mul,
    Div,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    Neg,
    Deserialize,
    Serialize,
    Default,
)]
pub struct Beats(i64);

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Add,
    Sub,
    Mul,
    Div,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    Default,
)]
/// Number of ticks per second.
pub struct Tempo(i64);

const TIME_BASE: i64 = 1476034560;

#[inline]
pub const fn whole_beat(beat: i64) -> Beats {
    Beats(beat * TIME_BASE)
}

impl Beats {
    pub const fn zero() -> Beats {
        Beats(0)
    }

    #[inline]
    pub fn from_float(b: f64) -> Beats {
        let whole = b.floor() as i64;
        let fractional = Beats(((b - whole as f64) * TIME_BASE as f64).round() as i64);
        assert!(fractional < Beats(TIME_BASE));

        whole_beat(whole) + fractional
    }

    #[inline]
    pub fn to_float(self) -> f64 {
        let whole = self.0 / TIME_BASE;
        let fractional = self.0 - (whole * TIME_BASE);
        whole as f64 + fractional as f64 / TIME_BASE as f64
    }

    #[inline]
    pub fn to_int_floored(self) -> i64 {
        self.0 / TIME_BASE
    }

    #[inline]
    pub fn abs(self) -> Beats {
        Beats(self.0.abs())
    }

    #[allow(non_snake_case)]
    pub const fn MAX() -> Beats {
        Beats(i64::MAX)
    }

    #[allow(non_snake_case)]
    pub const fn MIN() -> Beats {
        Beats(i64::MIN)
    }

    pub const fn multiply(self, x: i64) -> Beats {
        Beats(self.0 * x)
    }

    pub fn multiply_f(self, x: f64) -> Beats {
        Beats::from_float(self.to_float() * x)
    }

    #[inline]
    pub fn floor(self, numerator: i64, denominator: i64) -> Beats {
        if denominator == 0 || numerator == 0 {
            return self;
        };

        let dif = self.0 % ((TIME_BASE * numerator) / denominator);
        Beats(self.0 - dif)
    }

    #[inline]
    pub fn ceil(self, numerator: i64, denominator: i64) -> Beats {
        if denominator == 0 || numerator == 0 {
            return self;
        };

        self.floor(numerator, denominator) + Beats((TIME_BASE * numerator) / denominator)
    }

    pub fn snap(self, numerator: i64, denominator: i64) -> Beats {
        let floor = self.floor(numerator, denominator);
        let ceil = self.ceil(numerator, denominator);
        if (self.0 - floor.0).abs() < (ceil.0 - self.0).abs() {
            floor
        } else {
            ceil
        }
    }

    pub fn floor_fast<const NUMERATOR: i64, const DENOMINATOR: i64>(self) -> Beats {
        self.floor(NUMERATOR, DENOMINATOR)
    }

    pub fn ceil_fast<const NUMERATOR: i64, const DENOMINATOR: i64>(self) -> Beats {
        self.ceil(NUMERATOR, DENOMINATOR)
    }

    pub fn snap_fast<const NUMERATOR: i64, const DENOMINATOR: i64>(self) -> Beats {
        self.snap(NUMERATOR, DENOMINATOR)
    }

    pub fn snap_to_beats(self, beat: Beats) -> Beats {
        if beat > Beats::from_float(1.) {
            let numerator = beat.to_float() as i64;
            let denominator = 1;

            return match numerator {
                1 => self.snap_fast::<1, 1>(),
                2 => self.snap_fast::<2, 1>(),
                3 => self.snap_fast::<3, 1>(),
                4 => self.snap_fast::<4, 1>(),
                5 => self.snap_fast::<5, 1>(),
                6 => self.snap_fast::<6, 1>(),
                8 => self.snap_fast::<8, 1>(),
                _ => self.snap(numerator, denominator),
            };
        }

        let numerator = 1;
        let denominator = (1. / beat.to_float()) as i64;

        assert_eq!(numerator, 1); // This seems obvious but I see myself fucking it up and then the
                                  // hardcoded 1s are wrong.
        match denominator {
            1 => self.snap_fast::<1, 1>(),
            2 => self.snap_fast::<1, 4>(),
            3 => self.snap_fast::<1, 3>(),
            4 => self.snap_fast::<1, 4>(),
            5 => self.snap_fast::<1, 5>(),
            6 => self.snap_fast::<1, 6>(),
            8 => self.snap_fast::<1, 8>(),
            16 => self.snap_fast::<1, 16>(),
            32 => self.snap_fast::<1, 32>(),
            _ => self.snap(numerator, denominator),
        }
    }

    pub fn floor_to_beats(&self, beat: Beats) -> Beats {
        let numerator = 1;
        let denominator = (1. / beat.to_float()) as i64;

        assert_eq!(numerator, 1); // This seems obvious but I see myself fucking it up and then the
                                  // hardcoded 1s are wrong.
        match denominator {
            1 => self.floor_fast::<1, 1>(),
            2 => self.floor_fast::<1, 4>(),
            3 => self.floor_fast::<1, 3>(),
            4 => self.floor_fast::<1, 4>(),
            5 => self.floor_fast::<1, 5>(),
            6 => self.floor_fast::<1, 6>(),
            8 => self.floor_fast::<1, 8>(),
            16 => self.floor_fast::<1, 16>(),
            32 => self.floor_fast::<1, 32>(),
            _ => self.floor(numerator, denominator),
        }
    }

    pub fn raw(&self) -> i64 {
        self.0
    }

    pub fn divide(&self, x: i64) -> Beats {
        Beats(self.0 / x)
    }

    pub fn from_ticks(ticks: i64) -> Beats {
        Beats(ticks)
    }
}

impl std::cmp::PartialOrd for Beats {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}
impl std::cmp::Ord for Beats {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::fmt::Display for Beats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_float())
    }
}
impl std::fmt::Display for Tempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_bpm())
    }
}

impl Tempo {
    fn as_beats(&self) -> Beats {
        Beats(self.0)
    }

    pub fn from_bpm(arg: f64) -> Self {
        let beats = Beats::from_float(arg / 60.0);
        Tempo(beats.0)
    }

    pub fn to_bpm(&self) -> f64 {
        if self.0 == 0 {
            return 0.0;
        }

        let beats = self.as_beats();

        beats.to_float() * 60.0
    }

    pub fn beats_per_second(&self) -> f64 {
        let beats = self.as_beats();
        beats.to_float()
    }

    pub fn seconds_per_beat(&self) -> f64 {
        if self.0 == 0 {
            return 0.0;
        }

        let beats = self.as_beats();

        1. / beats.to_float()
    }

    pub fn seconds_of_beat(&self, beat: Beats) -> f64 {
        beat.to_float() * self.seconds_per_beat()
    }

    pub fn beats_of_seconds(&self, length_seconds: f64) -> Beats {
        Beats((self.0 as f64 * length_seconds) as i64)
    }
}
