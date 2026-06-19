#![no_std]
#![no_main]

use core::arch::asm;

use alloc::{format, string::String, vec::Vec};
use janglib::{self, print, println};

extern crate alloc;

pub const ABOUT: &'static str = r#"Jangri is a simple kernel inspired by xv6 and created by Aadarsh. It has no persistent filesystem, no support for SMP, no support for device detection. Although it's nearly useless, it showcases the author's low-level skills."#;

fn process(input: String) {
    let parts = input.split(" ").collect::<Vec<&str>>();

    if *parts.get(0).unwrap() == "echo" {
        if let Some(v) = parts.get(1) {
            println!("{}", v);
        } else {
            println!("echo WHAT??");
        }
    } else if *parts.get(0).unwrap() == "about" {
        println!("{}", ABOUT);
    } else {
        println!("Unrecognised command");
    }
}

#[unsafe(no_mangle)]
fn main() {
    loop {
        print!(">>> ");
        loop {
            let input = janglib::io::read();
            if input.is_empty() {
                continue;
            }
            process(input);
            println!("");
            break;
        }
    }
}
