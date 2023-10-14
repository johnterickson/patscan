#![feature(portable_simd )]
#![feature(generators, generator_trait)]
#![feature(test)]
extern crate test;

use std::{io::BufRead, ops::{Generator, GeneratorState}, pin::Pin};

const PAT_LEN: u8 = 52;

const PAT_CHARS_MAX_INDEX: u8 = 10 + 26 - 1;
fn pat_char_index(c: char) -> Option<u8> {
    match c {
        c if 'a' <= c && c <= 'z' => Some(c as u8 - 'a' as u8),
        c if '0' <= c && c <= '9' => Some(c as u8 - '0' as u8 + 26),
        _ => None
    }
}

pub fn sisd(line: &str) -> Vec<(usize, &str, usize)> {
    sisd_iter(line).collect()
}


pub fn sisd_iter(line: &str) -> impl Iterator<Item = (usize, &str, usize)> {
    GeneratorIteratorAdapter::new(sisd_inner(line))
}

pub fn sisd_inner(line: &str) -> impl Generator<Yield = (usize, &str, usize), Return = ()> + '_ {
    move || {
        let mut possible_start_index = 0;
        let mut confirmed_length: u8 = 0;
        let mut char_counts = [0u8; PAT_CHARS_MAX_INDEX as usize + 1];
        for (i, c) in line.char_indices() {
            if let Some(pat_char_index) = pat_char_index(c) {
                char_counts[pat_char_index as usize] += 1;
                confirmed_length += 1;

                if confirmed_length < PAT_LEN {
                    continue; // keep looking for more chars
                }

                const MAX_ESTIMATE: usize = (PAT_LEN as usize) * (PAT_LEN as usize);
                let mut sum = 0usize;
                for char_count in char_counts.iter_mut() {
                    let char_count = *char_count as usize;
                    sum += char_count * char_count;
                }
                let entropy = MAX_ESTIMATE - sum;
                // hex: entropy ~ 2482
                // random pat: entropy ~ 2584

                if entropy > 2525 {
                    let start_index = possible_start_index;
                    let end_index = possible_start_index + PAT_LEN as usize;
                    let substr = &line[start_index..end_index];
                    yield (start_index, substr, entropy);
                }

                possible_start_index += confirmed_length as usize;
            } else {
                possible_start_index = i + 1;
            }

            if confirmed_length > 0 {
                for char_count in char_counts.iter_mut() {
                    *char_count = 0;
                }
            }
            confirmed_length = 0;
        }
    }
}

pub fn simd(line: &str) -> Vec<(usize, &str, usize)> {
    let mut seen = std::collections::HashSet::new();
    simd_iter(line).filter(|i| seen.insert(i.0)).collect()
}

pub fn simd_iter(line: &str) -> impl Iterator<Item = (usize, &str, usize)> {
    GeneratorIteratorAdapter::new(simd_inner(line))
}

pub fn simd_inner(line: &str) -> impl Generator<Yield = (usize, &str, usize), Return = ()> + '_ {
    move || {
        use std::simd::*;

        assert!('0' < '9' && 'a' < 'z');

        const MATCH_LANES: usize = 8;

        const MIGHT_MISS_BEFORE: usize = MATCH_LANES - 1;
        const MIGHT_MISS_AFTER: usize = (PAT_LEN as usize - MIGHT_MISS_BEFORE) % MATCH_LANES;
        const BLOCKS_TO_MATCH: u8 = ((PAT_LEN as usize - MIGHT_MISS_BEFORE - MIGHT_MISS_AFTER) / MATCH_LANES) as u8;
        const _CHECK_FOR_ZERO: u8 = 1 / BLOCKS_TO_MATCH;

        const FREQ_BUCKETS: usize = 64;//PAT_CHARS_MAX_INDEX as usize + 1;

        let mut possible_start_block_index: usize = 0;
        let mut confirmed_blocks: u8 = 0;
        let mut counts: Simd<_,FREQ_BUCKETS> = Simd::splat(0u8);
        let mut char_blocks = line.as_bytes().chunks_exact(MATCH_LANES);
        while let Some(chunk_slice) = char_blocks.next() {

            let chunk: Simd<_,MATCH_LANES> = Simd::from_slice(chunk_slice);
            let number = chunk.simd_ge(Simd::splat('0' as u8)) & chunk.simd_le(Simd::splat('9' as u8));
            let lowercase = chunk.simd_ge(Simd::splat('a' as u8)) & chunk.simd_le(Simd::splat('z' as u8));
            
            if (number | lowercase).all() {

                confirmed_blocks += 1;

                let number_index = number.select(
                    chunk - Simd::splat('0' as u8),
                    Simd::splat(0));
                let lowercase_index = lowercase.select(
                    chunk - Simd::splat('a' as u8) + Simd::splat(10u8),
                    Simd::splat(0));
                let char_index = number_index | lowercase_index;

                // todo: simd this
                for char_index in char_index.as_array() {
                    counts[*char_index as usize] += 1;
                }

                if confirmed_blocks == BLOCKS_TO_MATCH {
                    const HIGHEST_SINGLE_COUNT : usize = (BLOCKS_TO_MATCH as usize) * (MATCH_LANES as usize);
                    const HIGHEST_ESTIMATE : usize = HIGHEST_SINGLE_COUNT * HIGHEST_SINGLE_COUNT;
                    let mut sum = 0usize;
                    for count in counts.as_array() {
                        let count = *count as usize;
                        sum += count * count;
                    }
                    let entropy = HIGHEST_ESTIMATE - sum;
                    // random PAT ~ 1528
                    // hex ~ 1480
                    if entropy > 1500 {
                        let start_index = (std::cmp::max(possible_start_block_index, 1) - 1) * MATCH_LANES;
                        let end_index = std::cmp::min(start_index + 2 * PAT_LEN as usize, line.len());
                        let substr = &line[start_index .. end_index];
                        for inner in sisd_iter(substr) {
                            yield (inner.0 + start_index, inner.1, inner.2);
                        }
                    }

                    possible_start_block_index += confirmed_blocks as usize;
                } else {
                    continue; //keep looking for more blocks
                }
            } else {
                possible_start_block_index += 1;
            }

            counts = Simd::splat(0u8);
            confirmed_blocks = 0;
        }

        let remainder = char_blocks.remainder();
        let start_index = line.len() - std::cmp::min(remainder.len() + PAT_LEN as usize - 1, line.len());
        for inner in sisd_iter(&line[start_index..]) {
            yield inner;
        }
    }
}

fn main() -> Result<(), std::io::Error> {
    let stdin = std::io::stdin().lock();
    for line in stdin.lines() {
        for (start_index, substr, entropy) in simd(&line?) {
            println!("{} {} {}", start_index, substr, entropy);
        }
    }

    Ok(())
}

// #![feature(test)]

// extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use test::Bencher;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref PAT_CHARS: Vec<char> = ('0'..='9').chain('a'..='z').collect();
        static ref LOWER_HEX_CHARS: Vec<char> = ('0'..='9').chain('a'..='f').collect();
        static ref NOT_PAT_CHARS: Vec<char> = ('A'..='Z').collect();
        static ref TEST_CHARS: Vec<char> = ('0'..='9').chain('a'..='z').chain('A'..='Z').collect();
        static ref NUMBERS: Vec<char> = ('0'..='9').collect();
    }

    fn random_pat() -> String {
        random_chars(&PAT_CHARS, PAT_LEN as usize)
    }

    fn random_chars(chars: &[char], count: usize) -> String {
        let mut rng = rand::thread_rng();
        let mut pat = String::with_capacity(count);
        for _ in 0..count {
            pat.push(chars[rng.gen_range(0..chars.len())]);
        }
        pat
    }

    #[test]
    fn match_direct() {
        let pat = random_pat();
        assert_eq!(sisd(&pat).len(), 1);
        assert_eq!(simd(&pat).len(), 1);
    }

    #[test]
    fn not_match_direct() {
        let pat = random_pat();
        let mut chars = pat.chars().collect::<Vec<_>>();
        chars[PAT_LEN as usize/2] = '$';
        let not_pat = String::from_iter(&chars);
        assert_eq!(sisd(&not_pat).len(), 0);
        assert_eq!(simd(&not_pat).len(), 0);
    }

    #[test]
    fn unlikely_pat() {
        let almost_pat = random_chars(&LOWER_HEX_CHARS, PAT_LEN as usize);
        assert_eq!(sisd(&almost_pat).len(), 0);
        assert_eq!(simd(&almost_pat).len(), 0);
    }

    #[test]
    fn long_not_pat() {
        let line = random_chars(&NOT_PAT_CHARS, 10000000);
        let line = line.as_str();
        assert_eq!(sisd(&line).len(), 0);
        assert_eq!(simd(&line).len(), 0);
    }

    #[test]
    fn long_hidden_in_not() {
        let pat = random_pat();
        let line = random_chars(&NOT_PAT_CHARS, 10_000) + &pat + &random_chars(&NOT_PAT_CHARS, 100);
        let line = &line;

        assert_eq!(sisd(&line).iter().next().map(|i| i.0), Some(10000));
        assert_eq!(simd(&line).iter().next().map(|i| i.0), Some(10000));
    }

    #[test]
    fn long_hidden_in_hex() {
        let pat = random_pat();
        let line = random_chars(&LOWER_HEX_CHARS, 10_000) + &pat + &random_chars(&LOWER_HEX_CHARS, 100);
        let line = &line;

        let found = sisd(&line).iter().next().unwrap().0;
        assert!(found.abs_diff(10000) < PAT_LEN.into(), "{}", found);
        let found = sisd(&line).iter().next().unwrap().0;
        assert!(found.abs_diff(10000) < PAT_LEN.into(), "{}", found);
    }

    #[bench]
    fn bench_sisd_none(b: &mut Bencher) {
        let line = random_chars(&NOT_PAT_CHARS, 100_000);
        let line = line.as_str();
        test::black_box(line);

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                assert_eq!(sisd(line).len(), 0);
            }
        });
    }

    #[bench]
    fn bench_simd_none(b: &mut Bencher) {
        let line = random_chars(&NOT_PAT_CHARS, 100_000);
        let line = line.as_str();
        test::black_box(line);

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                assert_eq!(simd(line).len(), 0);
            }
        });
    }

    #[bench]
    fn bench_sisd_hidden_in_hex(b: &mut Bencher) {
        let pat = random_pat();
        let line = random_chars(&LOWER_HEX_CHARS, 100_000) + &pat + &random_chars(&LOWER_HEX_CHARS, 1000);
        let line = &line;

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                let found = sisd(line);
                for f in found {
                    assert!(f.0.abs_diff(100_000) < PAT_LEN.into(), "{:?}", f);
                }
            }
        });
    }

    #[bench]
    fn bench_simd_hidden_in_hex(b: &mut Bencher) {
        let pat = random_pat();
        let line = random_chars(&LOWER_HEX_CHARS, 100_000) + &pat + &random_chars(&LOWER_HEX_CHARS, 1000);
        let line = &line;

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                let found = simd(line);
                for f in found {
                    assert!(f.0.abs_diff(100_000) < PAT_LEN.into(), "{:?}", f);
                }
            }
        });
    }
}


// https://stackoverflow.com/questions/16421033/lazy-sequence-generation-in-rust
struct GeneratorIteratorAdapter<G>(Pin<Box<G>>);

impl<G> GeneratorIteratorAdapter<G>
where
    G: Generator<Return = ()>,
{
    fn new(gen: G) -> Self {
        Self(Box::pin(gen))
    }
}

impl<G> Iterator for GeneratorIteratorAdapter<G>
where
    G: Generator<Return = ()>,
{
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.as_mut().resume(()) {
            GeneratorState::Yielded(x) => Some(x),
            GeneratorState::Complete(_) => None,
        }
    }
}