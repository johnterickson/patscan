
Uses SIMD (e.g. AVX) to quickly find PATs

PATs are 52 chars [0-9a-z].

1. Find a contiguous block of 52 (or a little less) [0-9a-z]
2. if any character in the block is non-PAT, then move on. This needs to be fast.
3. if we have a substring that is all PAT-like, then estimate the entropy.
   We do this by looking at the frequency distribution.
   We count the frequency of each character.  We then square the count.
   Finally we sum the squares.  A lower sum has more entropy (more PAT-like).
   11...11 will result in a frequency histogram of [0,52,0,..]
    this is ~50^2 = 2500
   a pat will look more like a flat frequency histogram [1,2,1,1,...]
    so ~1.5^2 * 52 = 100
   This even helps with lowercase hex [0-9a-f] as that will be lopsided
    [2,2,3,.., 0, 0, 0, ...]
   lowercase hex could technically be a pat, but we use the entropy meausre to
   rule it out.

Results:
SIMD non-pat-like text (all uppercase)
  14 us / 100KB = 80 Gbps
SIMD pat-like text but not a PAT (lowercase hex)
  97 us / 100KB = 8 Gbps

non-SIMD are 8x and 4x slower respectively

TODO: don't frequency count until we have all 52 chars confirmed as PAT-like
TODO: SIMD update frequency counts