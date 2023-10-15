use std::io::{BufReader, BufRead};

use patscan_lib::simd;


fn main() -> Result<(), std::io::Error> {
    let stdin = BufReader::new(std::io::stdin().lock());
    for line in stdin.lines() {
        if let Some((start_index, substr, entropy)) = simd(&line?) {
            println!("{} {} {}", start_index, substr, entropy);
        }
    }

    Ok(())
}
