using System;
using System.Security.Cryptography;
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using patscan;
using patscan_test;

namespace PatScanBenchBenchmarks
{
    public class PatScanBench
    {
        private string PatInNonPatCharsTestString;
        private string PatInPatCharsTestString;

        [Params(1_000, 10_000, 100_000)]
        public int N;

        [GlobalSetup]
        public void Setup()
        {
             PatInNonPatCharsTestString = 
                PatScanTests.RandomChars(PatScanTests.Uppercase, N)
                + PatScanTests.RandomPAT()
                + PatScanTests.RandomChars(PatScanTests.Uppercase, 1_000);
            PatInPatCharsTestString = 
                PatScanTests.RandomChars(PatScanTests.HexLower, N)
                + PatScanTests.RandomPAT()
                + PatScanTests.RandomChars(PatScanTests.HexLower, 1_000);
        }

        [Benchmark]
        public long PatInNonPatChars() => PatScan.call_simd(PatInNonPatCharsTestString);

        [Benchmark]
        public long PatInPatChars() => PatScan.call_simd(PatInPatCharsTestString);
    }

    public class Program
    {
        public static void Main(string[] args)
        {
            var summary = BenchmarkRunner.Run<PatScanBench>();
        }
    }
}