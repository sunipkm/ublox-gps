use std::ops::{Add, Deref, Div, Mul, Neg, Sub};

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::{ubx::Frequency, GnssFreq, GnssSatellite, SatPathInfo, UbxGpsInfo};

use num_traits::{Float, Inv, Num, NumAssignOps, NumAssignRef, NumCast, NumOps, ToPrimitive};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Uncertain<T>(T, T);

impl<T: Num + NumCast + ToPrimitive + NumOps + NumAssignOps + NumAssignRef + Neg + Copy> Uncertain<T> {
    pub fn new(value: T, uncertainty: T) -> Self {
        Uncertain(value, uncertainty)
    }

    pub fn value(&self) -> T {
        self.0
    }

    pub fn error(&self) -> T {
        self.1
    }
}

impl <T: Num> From<T> for Uncertain<T>
{
    fn from(value: T) -> Self {
        Uncertain(value, T::zero())
    }
}

impl <T: NumCast + Copy> Uncertain<T> {
    pub fn cast_into<U>(&self) -> Uncertain<U>
    where
        U: NumCast + Copy,
    {
        Uncertain(NumCast::from(self.0).unwrap(), NumCast::from(self.1).unwrap())
    }
}

impl <T: Num + ToPrimitive + NumOps + NumCast> Add for Uncertain<T> {
    type Output = Uncertain<T>;

    fn add(self, other: Uncertain<T>) -> Uncertain<T> {
        let stdev1: f64 = NumCast::from(self.1).unwrap();
        let stdev2: f64 = NumCast::from(other.1).unwrap();
        let stdev = (stdev1 * stdev1 + stdev2 * stdev2).sqrt();
        Uncertain(self.0 + other.0, NumCast::from(stdev).unwrap())
    }
}

impl <T: Num + NumOps + Sub> Neg for Uncertain<T> {
    type Output = Uncertain<T>;

    fn neg(self) -> Uncertain<T> {
        Uncertain(T::zero() - self.0, self.1)
    }
}

impl <T: Num + ToPrimitive + NumOps + NumCast> Sub for Uncertain<T> {
    type Output = Uncertain<T>;

    fn sub(self, other: Uncertain<T>) -> Uncertain<T> {
        self + (-other)
    }
}

impl <T: Num + ToPrimitive + NumOps + NumCast + Copy> Mul for Uncertain<T> {
    type Output = Uncertain<T>;

    fn mul(self, other: Uncertain<T>) -> Uncertain<T> {
        let v1: f64 = NumCast::from(self.0).unwrap();
        let v2: f64 = NumCast::from(other.0).unwrap();
        let u1: f64 = NumCast::from(self.1).unwrap();
        let u2: f64 = NumCast::from(other.1).unwrap();
        let err = ((u1/v1) * (u1/v1) + (u2/v2) * (u2/v2)).sqrt();
        Uncertain(self.0 * other.0, NumCast::from(err).unwrap())
    }
}

impl <T: Num + ToPrimitive + NumOps + NumCast + Copy> Inv for Uncertain<T> {
    type Output = Uncertain<T>;

    fn inv(self) -> Uncertain<T> {
        let v: f64 = NumCast::from(self.0).unwrap();
        let u: f64 = NumCast::from(self.1).unwrap();
        let err = (u/(v*v)).abs();
        Uncertain(T::one() / self.0, NumCast::from(err).unwrap())
    }
}

impl <T: Num + ToPrimitive + NumOps + NumCast + Copy> Div for Uncertain<T> {
    type Output = Uncertain<T>;

    fn div(self, other: Uncertain<T>) -> Uncertain<T> {
        let v1: f64 = NumCast::from(self.0).unwrap();
        let v2: f64 = NumCast::from(other.0).unwrap();
        let u1: f64 = NumCast::from(self.1).unwrap();
        let u2: f64 = NumCast::from(other.1).unwrap();
        let err = ((u1/v1) * (u1/v1) + (u2/v2) * (u2/v2)).sqrt();
        Uncertain(self.0 / other.0, NumCast::from(err).unwrap())
    }
}

fn factor(f1: f64, f2: f64) -> f64 {
    const K: f64 = 1e-16 / 40.308;
    let a = f1 * f1;
    let b = f2 * f2;
    K * a * b / (a - b)
}

struct TecData {
    source: GnssSatellite,
    pointing: (u16, u8),
    channels: (GnssFreq, GnssFreq),
    phase_tec: Option<Uncertain<f64>>,
    range_tec: Option<Uncertain<f64>>,
}

struct TecInfo {
    timestamp: DateTime<Utc>,
    location: (f64, f64, f32),
    tec: Vec<TecData>,
}

#[derive(Debug, Error)]
pub enum TecError {
    #[error("Invalid number of carrier phase measurements")]
    InvalidCarrierPhase,
}

impl TecInfo {
    pub fn assimilate(src: &UbxGpsInfo) -> Self {
        let timestamp = src.timestamp();
        let location = src.location();
        let mut tec = Vec::new();
        for (sat, ch) in src.carrier_phase() {
            if ch.meas.len() != 2 {
                continue;
            }
            let m0 = &ch.meas[0];
            let m1 = &ch.meas[1];
            let channels = (m0.channel, m1.channel);
            let f1 = m0.channel.get_freq();
            let f2 = m1.channel.get_freq();
            let fac = factor(m0.channel.get_freq(), m1.channel.get_freq());
            let fac = Uncertain::new(fac, 0.0);
            if let Some((m0_phase, m0_perr)) = m0.carrier_phase {
                if let Some((m1_phase, m1_perr)) = m1.carrier_phase {

                }


            }
        }
        TecInfo {
            timestamp,
            location,
            tec,
        }
    }
}