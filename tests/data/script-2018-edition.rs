//! ```cargo
//! [package]
//! edition = "2018"
//!
//! [dependencies]
//! boolinator = "=0.1.0"
//! ```

use boolinator::Boolinator;

fn main() {
    println!("--output--");
    println!("{:?}", true.as_some(1));
}
