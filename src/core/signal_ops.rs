use std::cmp::Ordering;

use itertools::Itertools;

use crate::core::signal::RealSignal;
pub use crate::prelude::*;

fn permissive_max<T: SignalType>(a: &&T, b: &&T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Greater)
}

impl<T: SignalType> RealSignal<T> {
    pub fn normalize(&mut self) {
        let max = *self
            .iter()
            .filter(|x| x.is_finite())
            .max_by(permissive_max)
            .expect("Normalize() called on empty RealSignal<T>");
        // Don't think too hard about it
        *self = &*self / max;
    }
}
impl<T: SignalType> Signal<T> {
    pub fn normalize(&mut self) {
        let max = *self
            .abs()
            .iter()
            .filter(|x| x.is_finite())
            .max_by(permissive_max)
            .expect("Normalize() called on empty Signal<T>");
        // println!("maxi: {max}");
        // Don't think too hard about it
        *self = &*self / max;
    }
}

impl<T: SignalType> Signal<T> {
    pub fn abs(&self) -> Vec<T> {
        self.iter().map(|x| x.norm()).collect_vec()
    }
    pub fn re(&self) -> Vec<T> {
        self.iter().map(|x| x.re).collect_vec()
    }
    pub fn im(&self) -> Vec<T> {
        self.iter().map(|x| x.im).collect_vec()
    }
    pub fn conj(mut self) -> Self {
        self.iter_mut().for_each(|x| *x = x.conj());
        self
    }
}

impl<T: SignalType> std::ops::Mul for &Signal<T> {
    type Output = Signal<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = Signal::from_vec(
            self.sample_rate,
            self.iter()
                .zip(rhs.iter())
                .map(|(a, b)| a * b)
                .collect_vec(),
        );
        out.time = i64::max(self.time, rhs.time);
        out
    }
}
impl<T: SignalType> std::ops::Add for &Signal<T> {
    type Output = Signal<T>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut out = Signal::from_vec(
            self.sample_rate,
            self.iter()
                .zip(rhs.iter())
                .map(|(a, b)| a + b)
                .collect_vec(),
        );
        out.time = i64::max(self.time, rhs.time);
        out
    }
}
impl<T: SignalType> std::ops::AddAssign for Signal<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a += b);
    }
}

impl<T: SignalType> std::ops::MulAssign for Signal<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a *= b);
    }
}

// Scalar, Signal
impl<T: SignalType> std::ops::Mul<T> for &Signal<T> {
    type Output = Signal<T>;

    fn mul(self, rhs: T) -> Self::Output {
        let mut out =
            Signal::from_vec(self.sample_rate, self.iter().map(|a| a * rhs).collect_vec());
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Div<T> for &Signal<T> {
    type Output = Signal<T>;

    fn div(self, rhs: T) -> Self::Output {
        let mut out =
            Signal::from_vec(self.sample_rate, self.iter().map(|a| a / rhs).collect_vec());
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Add<T> for &Signal<T> {
    type Output = Signal<T>;

    fn add(self, rhs: T) -> Self::Output {
        let mut out =
            Signal::from_vec(self.sample_rate, self.iter().map(|a| a + rhs).collect_vec());
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Sub<T> for &Signal<T> {
    type Output = Signal<T>;

    fn sub(self, rhs: T) -> Self::Output {
        let mut out =
            Signal::from_vec(self.sample_rate, self.iter().map(|a| a - rhs).collect_vec());
        out.time = self.time;
        out
    }
}

// Scalar, RealSignal
impl<T: SignalType> std::ops::Mul<T> for &RealSignal<T> {
    type Output = RealSignal<T>;

    fn mul(self, rhs: T) -> Self::Output {
        let mut out = RealSignal::from_vec(
            self.sample_rate,
            self.iter().map(|a| *a * rhs).collect_vec(),
        );
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Div<T> for &RealSignal<T> {
    type Output = RealSignal<T>;

    fn div(self, rhs: T) -> Self::Output {
        let mut out = RealSignal::from_vec(
            self.sample_rate,
            self.iter().map(|a| *a / rhs).collect_vec(),
        );
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Add<T> for &RealSignal<T> {
    type Output = RealSignal<T>;

    fn add(self, rhs: T) -> Self::Output {
        let mut out = RealSignal::from_vec(
            self.sample_rate,
            self.iter().map(|a| *a + rhs).collect_vec(),
        );
        out.time = self.time;
        out
    }
}
impl<T: SignalType> std::ops::Sub<T> for &RealSignal<T> {
    type Output = RealSignal<T>;

    fn sub(self, rhs: T) -> Self::Output {
        let mut out = RealSignal::from_vec(
            self.sample_rate,
            self.iter().map(|a| *a - rhs).collect_vec(),
        );
        out.time = self.time;
        out
    }
}

// Vector, RealSignal
impl<T: SignalType> std::ops::Mul for &RealSignal<T> {
    type Output = RealSignal<T>;

    fn mul(self, rhs: &RealSignal<T>) -> Self::Output {
        let mut out = RealSignal::from_vec(
            self.sample_rate,
            self.iter()
                .zip(rhs.iter())
                .map(|(a, b)| *a * *b)
                .collect_vec(),
        );
        out.time = self.time;
        out
    }
}
