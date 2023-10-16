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
        private static readonly string PatInNonPatCharsTestString;
        private static readonly string PatInPatCharsTestString;


        static PatScanBench()
        {
            PatInNonPatCharsTestString = 
                PatScanTests.RandomChars(PatScanTests.Uppercase, 100_000)
                + PatScanTests.RandomPAT()
                + PatScanTests.RandomChars(PatScanTests.Uppercase, 1_000);
            PatInPatCharsTestString = 
                PatScanTests.RandomChars(PatScanTests.HexLower, 100_000)
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