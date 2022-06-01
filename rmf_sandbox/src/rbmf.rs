// RBMF stands for "RMF Building Map Format"

use bevy_egui::egui::emath::Numeric;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct RbmfString(usize, pub String);

impl From<String> for RbmfString {
    fn from(s: String) -> Self {
        RbmfString(1, s)
    }
}

impl Default for RbmfString {
    fn default() -> Self {
        Self(1, "".to_string())
    }
}

impl PartialEq for RbmfString {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl From<RbmfString> for String {
    fn from(s: RbmfString) -> Self {
        s.1
    }
}

impl Deref for RbmfString {
    type Target = String;
    fn deref(&self) -> &String {
        &self.1
    }
}

impl DerefMut for RbmfString {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.1
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct RbmfInt(usize, pub i64);

impl From<i64> for RbmfInt {
    fn from(i: i64) -> Self {
        RbmfInt(2, i)
    }
}

impl Default for RbmfInt {
    fn default() -> Self {
        Self(2, 0)
    }
}

impl PartialEq for RbmfInt {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl PartialOrd for RbmfInt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl From<RbmfInt> for i64 {
    fn from(i: RbmfInt) -> Self {
        i.1
    }
}

impl Deref for RbmfInt {
    type Target = i64;
    fn deref(&self) -> &i64 {
        &self.1
    }
}

impl DerefMut for RbmfInt {
    fn deref_mut(&mut self) -> &mut i64 {
        &mut self.1
    }
}

impl Numeric for RbmfInt {
    const INTEGRAL: bool = true;
    const MIN: Self = Self(2, i64::MIN);
    const MAX: Self = Self(2, i64::MAX);

    fn to_f64(self) -> f64 {
        self.1 as f64
    }

    fn from_f64(num: f64) -> Self {
        Self(2, num as i64)
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct RbmfFloat(usize, pub f64);

impl From<f64> for RbmfFloat {
    fn from(f: f64) -> Self {
        Self(3, f)
    }
}

impl Default for RbmfFloat {
    fn default() -> Self {
        Self(3, 0.)
    }
}

impl PartialEq for RbmfFloat {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl PartialOrd for RbmfFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl From<RbmfFloat> for f64 {
    fn from(f: RbmfFloat) -> Self {
        f.1
    }
}

impl Deref for RbmfFloat {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.1
    }
}

impl DerefMut for RbmfFloat {
    fn deref_mut(&mut self) -> &mut f64 {
        &mut self.1
    }
}

impl Numeric for RbmfFloat {
    const INTEGRAL: bool = false;
    const MIN: Self = Self(3, f64::MIN);
    const MAX: Self = Self(3, f64::MAX);

    fn to_f64(self) -> f64 {
        self.1
    }

    fn from_f64(num: f64) -> Self {
        Self(3, num)
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RbmfBool(usize, pub bool);

impl From<bool> for RbmfBool {
    fn from(b: bool) -> Self {
        Self(4, b)
    }
}

impl Default for RbmfBool {
    fn default() -> Self {
        Self(4, false)
    }
}

impl PartialEq for RbmfBool {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl From<RbmfBool> for bool {
    fn from(b: RbmfBool) -> Self {
        b.1
    }
}

impl Deref for RbmfBool {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.1
    }
}

impl DerefMut for RbmfBool {
    fn deref_mut(&mut self) -> &mut bool {
        &mut self.1
    }
}
