use std::ops::Add;
pub mod config;
#[macro_use]
pub(self) mod macros;
pub mod satacc;
pub mod sim;
pub(self) mod test_utils;
// from now all code are for test
#[no_mangle]
pub extern "C" fn say_hello(point: &Point, rect: &Rec) {
    println!("Hello, world! :{point:?} {rect:?}");
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct Rec {
    x: i32,
    y: i32,
}
impl Rec {
    pub fn new(x: i32, y: i32) -> Rec {
        Rec { x, y }
    }
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
    #[no_mangle]
    pub extern "C" fn get_x(&self) -> i32 {
        self.x
    }
    #[no_mangle]
    pub extern "C" fn get_y(&self) -> i32 {
        self.y
    }
    #[no_mangle]
    pub extern "C" fn set_x(&mut self, x: i32) {
        self.x = x;
    }
    #[no_mangle]
    pub extern "C" fn set_y(&mut self, y: i32) {
        self.y = y;
    }
}
impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[cfg(test)]
mod test {
    // use super::*;
    // use satacc::Simulator;
    #[test]
    fn test_simulator() {
        // let _sim = Simulator::new("test_config.toml");
    }
}
