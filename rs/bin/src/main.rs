use std::io::{BufReader, BufRead};

use patscan_lib::simd;
use widestring::{U16String, U16Str};


fn main() -> Result<(), std::io::Error> {
    let stdin = BufReader::new(std::io::stdin().lock());
    for line in stdin.lines() {
        let line = U16String::from_str(&line?);
        if let Some((start_index, substr, entropy)) = simd(line.as_slice()) {
            let substr = U16Str::from_slice(substr);
            let substr = substr.to_string().unwrap();
            println!("{} {} {}", start_index, substr, entropy);
        }
    }

    Ok(())
}
