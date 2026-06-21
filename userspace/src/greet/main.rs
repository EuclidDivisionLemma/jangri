#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use janglib;
use janglib::{print, println};

#[unsafe(no_mangle)]
fn main() {
    println!("Hello Stranger!");
}
