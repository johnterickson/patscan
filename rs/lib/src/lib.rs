#![feature(portable_simd )]
#![feature(test)]

use widestring::U16CStr;

#[cfg(test)]
extern crate test;

const PAT_LEN: u8 = 52;
const PAT_CHARS: usize = 10 + 26;
fn pat_char_index(c: u16) -> Option<u8> {
    match c {
        c if 'a' as u16 <= c && c <= 'z' as u16 => Some((c - 'a' as u16 + 10) as u8),
        c if '0' as u16 <= c && c <= '9' as u16 => Some((c - '0' as u16) as u8),
        _ => None
    }
}

const FREQ_COMPRESSION: u8 = 1;
const FREQ_BUCKETS: usize = (PAT_CHARS + FREQ_COMPRESSION as usize - 1)  / (FREQ_COMPRESSION as usize);
const EXPECTED_AVERAGE_CHAR: usize = (
    ( (FREQ_BUCKETS - 1) / 2 * FREQ_COMPRESSION as usize) +
    ( (FREQ_BUCKETS + 0) / 2 * FREQ_COMPRESSION as usize)
) / 2;
const NORMALIZED_SCALING_MULTIPLE: usize = 1;
const LOPSIDEDNESS_SCALING_DIVISOR: usize = 1;

// pat ~ 22 - 1 = 21
// hex ~ 10 - 8 = 2
// every_nth ~ 8 - 1 = 7
const PAT_ENTROPY_MINIMUM: usize = 8;

fn approx_entropy(char_counts: &[u8; FREQ_BUCKETS]) -> usize {
    // two measures:
    // 1. frequency distribution
    //    flatter distribution --> more random
    // 2. lopsidedness (e.g. all hex will be very lopsided)
    //    more lopsided --> less random
    let mut total_count = 0;
    let mut weighted_total = 0;
    let mut sum_squared = 0;
    for (ch, count) in char_counts.iter().enumerate() {
        let count = *count as usize;
        total_count += count;
        sum_squared += count * count;
        weighted_total += ch * FREQ_COMPRESSION as usize * count;
    }
    let max = std::cmp::min(PAT_LEN as usize, total_count);
    let max = max * max;
    let normalized = max * NORMALIZED_SCALING_MULTIPLE / sum_squared;

    let average_char = weighted_total / total_count;
    let lopsidedness = average_char.abs_diff(EXPECTED_AVERAGE_CHAR);
    let lopsidedness = lopsidedness / LOPSIDEDNESS_SCALING_DIVISOR;

    let merged = normalized.saturating_sub(lopsidedness);
    // dbg!(char_counts, max, sum_squared, normalized, average_char, lopsidedness, merged);
    merged
}

pub fn sisd(line: &[u16]) -> Option<(usize, &[u16], usize)> {
    let mut possible_start_index = 0;
    let mut confirmed_length: u8 = 0;
    let mut char_counts = [0u8; FREQ_BUCKETS];
    for (i, c) in line.iter().enumerate() {
        if let Some(pat_char_index) = pat_char_index(*c) {
            let pat_char_index = pat_char_index / FREQ_COMPRESSION;
            char_counts[pat_char_index as usize] += 1;
            confirmed_length += 1;

            if confirmed_length < PAT_LEN {
                continue; // keep looking for more chars
            }

            let entropy = approx_entropy(&char_counts);
            if entropy >= PAT_ENTROPY_MINIMUM  {
                let start_index = possible_start_index;
                let end_index = possible_start_index + PAT_LEN as usize;
                let substr = &line[start_index..end_index];
                return Some((start_index, substr, entropy))
            }

            possible_start_index += confirmed_length as usize;
        } else {
            possible_start_index = i + 1;
        }

        if confirmed_length > 0 {
            char_counts.iter_mut().for_each(|c| *c = 0);
        }
        confirmed_length = 0;
    }

    None
}

#[no_mangle]
pub extern fn simd_c(str: *const u16, len: u32) -> u32 {
    let line = unsafe { U16CStr::from_ptr_unchecked(str, len as usize) };
    simd(line.as_slice()).map(|i| i.0 as u32).unwrap_or(u32::MAX)
}


pub fn simd(line: &[u16]) -> Option<(usize, &[u16], usize)> {
    use std::simd::*;

    // println!("{}", line);
    assert!('0' < '9' && 'a' < 'z');

    const MATCH_LANES: usize = 8;

    const MIGHT_MISS_BEFORE: usize = MATCH_LANES - 1;
    const MIGHT_MISS_AFTER: usize = (PAT_LEN as usize - MIGHT_MISS_BEFORE) % MATCH_LANES;
    const BLOCKS_TO_MATCH: u8 = ((PAT_LEN as usize - MIGHT_MISS_BEFORE - MIGHT_MISS_AFTER) / MATCH_LANES) as u8;
    const _CHECK_FOR_ZERO: u8 = 1 / BLOCKS_TO_MATCH;

    let mut possible_start_block_index: usize = 0;
    let mut confirmed_blocks: u8 = 0;
    let mut char_counts = [0u8; FREQ_BUCKETS];
    let mut char_blocks = line.chunks_exact(MATCH_LANES);
    while let Some(chunk_slice) = char_blocks.next() {

        let chunk: Simd<_,MATCH_LANES> = Simd::from_slice(chunk_slice);
        let number = chunk.simd_ge(Simd::splat('0' as u16)) & chunk.simd_le(Simd::splat('9' as u16));
        let lowercase = chunk.simd_ge(Simd::splat('a' as u16)) & chunk.simd_le(Simd::splat('z' as u16));
        
        if (number | lowercase).all() {

            confirmed_blocks += 1;

            let number_index = number.select(
                chunk - Simd::splat('0' as u16),
                Simd::splat(0));
            let lowercase_index = lowercase.select(
                chunk - Simd::splat('a' as u16) + Simd::splat(10u16),
                Simd::splat(0));
            let char_index = number_index | lowercase_index;
            let char_index = char_index / Simd::splat(FREQ_COMPRESSION as u16);

            // todo: simd this
            for char_index in char_index.as_array() {
                char_counts[*char_index as usize] += 1;
            }

            if confirmed_blocks == BLOCKS_TO_MATCH {
                let entropy = approx_entropy(&char_counts);
                if entropy >= PAT_ENTROPY_MINIMUM  {
                    let start_index = (std::cmp::max(possible_start_block_index, 1) - 1) * MATCH_LANES;
                    let end_index = std::cmp::min(start_index + 2 * PAT_LEN as usize, line.len());
                    let substr = &line[start_index .. end_index];
                    return sisd(substr).map(|i| (i.0 + start_index, i.1, i.2));
                }

                possible_start_block_index += confirmed_blocks as usize;
            } else {
                continue; //keep looking for more blocks
            }
        } else {
            possible_start_block_index += 1;
        }

        if confirmed_blocks > 0 {
            char_counts.iter_mut().for_each(|c| *c = 0);
        }
        confirmed_blocks = 0;
    }

    let remainder = char_blocks.remainder();
    let start_index = line.len() - std::cmp::min(remainder.len() + PAT_LEN as usize - 1, line.len());
    return sisd(&line[start_index..]).map(|i| (i.0 + start_index, i.1, i.2));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use test::Bencher;
    use lazy_static::lazy_static;
    use widestring::U16String;
    
    // Lower numbers cause tests to fail as unlucky PATs overlap with lucky not-PATs
    const NTH: u8 = 5;

    #[cfg(debug_assertions)]
    const LOTS: usize = 1_000;

    #[cfg(not(debug_assertions))]
    const LOTS: usize = 100_000;

    lazy_static! {
        static ref PAT_CHARS: Vec<char> = ('0'..='9').chain('a'..='z').collect();
        static ref EVERY_NTH_PAT_CHARS: Vec<char> = ('0'..='9').chain('a'..='z').filter(|c| *c as u8 % NTH == 0).collect();
        static ref LOWER_HEX_CHARS: Vec<char> = ('0'..='9').chain('a'..='f').collect();
        static ref NOT_PAT_CHARS: Vec<char> = ('A'..='Z').collect();
        static ref TEST_CHARS: Vec<char> = ('0'..='9').chain('a'..='z').chain('A'..='Z').collect();
        static ref NUMBERS: Vec<char> = ('0'..='9').collect();
    }

    fn random_pat() -> U16String {
        random_chars(&PAT_CHARS, PAT_LEN as usize)
    }

    fn random_chars(chars: &[char], count: usize) -> U16String {
        let mut rng = rand::thread_rng();
        let mut pat = U16String::with_capacity(count);
        for _ in 0..count {
            pat.push_char(chars[rng.gen_range(0..chars.len())]);
        }
        pat
    }

    #[test]
    fn match_direct() {
        let pat_str = random_pat();
        let pat = pat_str.as_slice();
    
        assert_eq!(sisd(pat).map(|i| i.0), Some(0), "{}", pat_str.to_string_lossy());
        assert_eq!(simd(pat).map(|i| i.0), Some(0), "{}", pat_str.to_string_lossy());
    }

    #[test]
    fn match_direct_lots() {
        for _ in 0..LOTS {
            match_direct();
        }
    }

    #[test]
    fn not_match_direct() {
        let pat = random_pat();
        
        let mut chars = pat.chars().map(|c| c.unwrap()).collect::<Vec<_>>();
        chars[PAT_LEN as usize/2] = '$';
        let not_pat = U16String::from_iter(&chars);
        let not_pat = not_pat.as_slice();

        assert_eq!(sisd(&not_pat), None);
        assert_eq!(simd(&not_pat), None);
    }

    #[test]
    fn not_match_direct_lots() {
        for _ in 0..LOTS {
            not_match_direct();
        }
    }

    #[test]
    fn unlikely_pat_hex() {
        let almost_pat = random_chars(&LOWER_HEX_CHARS, PAT_LEN as usize);
        let almost_pat = almost_pat.as_slice();
        assert_eq!(sisd(&almost_pat), None);
        assert_eq!(simd(&almost_pat), None);
    }

    #[test]
    fn unlikely_pat_hex_lots() {
        for _ in 0..LOTS {
            unlikely_pat_hex();
        }
    }

    #[test]
    fn unlikely_pat_every_nth() {
        let almost_pat = random_chars(&EVERY_NTH_PAT_CHARS, PAT_LEN as usize);
        let almost_pat = almost_pat.as_slice();
        assert_eq!(sisd(&almost_pat), None);
        assert_eq!(simd(&almost_pat), None);
    }

    #[test]
    fn unlikely_pat_every_nth_lots() {
        for _ in 0..LOTS {
            unlikely_pat_every_nth();
        }
    }

    #[test]
    fn long_not_pat() {
        let line = random_chars(&NOT_PAT_CHARS, 10000000);
        let line = line.as_slice();
        assert_eq!(sisd(&line), None);
        assert_eq!(simd(&line), None);
    }

    #[test]
    fn long_hidden_in_not() {
        let pat = random_pat();
        let mut line = random_chars(&NOT_PAT_CHARS, 10_000);
        line.push(&pat);
        line.push(random_chars(&NOT_PAT_CHARS, 100));
        let line = line.as_slice();

        // assert_eq!(sisd(&line).iter().next().map(|i| i.0), Some(10000));
        assert_eq!(simd(&line).iter().next().map(|i| i.0), Some(10000));
    }

    #[test]
    fn long_hidden_in_hex() {
        let pat = random_pat();
        let mut line = random_chars(&LOWER_HEX_CHARS, 10_000);
        line.push(&pat);
        line.push(random_chars(&LOWER_HEX_CHARS, 100));
        let line = line.as_slice();

        let found = sisd(&line).iter().next().unwrap().0;
        assert!(found.abs_diff(10000) < PAT_LEN.into(), "{}", found);
        let found = sisd(&line).iter().next().unwrap().0;
        assert!(found.abs_diff(10000) < PAT_LEN.into(), "{}", found);
    }

    #[bench]
    fn bench_sisd_none(b: &mut Bencher) {
        let line = random_chars(&NOT_PAT_CHARS, 100_000);
        let line = line.as_slice();
        test::black_box(line);

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                assert_eq!(sisd(line), None);
            }
        });
    }

    #[bench]
    fn bench_simd_none(b: &mut Bencher) {
        let line = random_chars(&NOT_PAT_CHARS, 100_000);
        let line = line.as_slice();
        test::black_box(line);

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                assert_eq!(simd(line), None);
            }
        });
    }

    #[bench]
    fn bench_sisd_hidden_in_hex(b: &mut Bencher) {
        let pat = random_pat();
        let mut line = random_chars(&LOWER_HEX_CHARS, 100_000);
        line.push(&pat);
        line.push(random_chars(&LOWER_HEX_CHARS, 1000));
        let line = line.as_slice();

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                let found = sisd(line);
                if let Some(f) = found {
                    assert!(f.0.abs_diff(100_000) < PAT_LEN.into(), "{:?}", f);
                }
            }
        });
    }

    #[bench]
    fn bench_simd_hidden_in_hex(b: &mut Bencher) {
        let pat = random_pat();
        let mut line = random_chars(&LOWER_HEX_CHARS, 100_000);
        line.push(&pat);
        line.push(random_chars(&LOWER_HEX_CHARS, 1000));
        let line = line.as_slice();

        b.iter(|| {
            // Inner closure, the actual test
            for _ in 1..=1 {
                let found = simd(line);
                if let Some(f) = found {
                    assert!(f.0.abs_diff(100_000) < PAT_LEN.into(), "{:?}", f);
                }
            }
        });
    }
}