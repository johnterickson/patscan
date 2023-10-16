namespace patscan_test;
using patscan;

[TestClass]
public class PatScanTests
{
    public const int PatLength = 52;

    public static readonly List<char> Numbers = Enumerable.Range(0, 256)
        .Select(i => (char)i)
        .Where(c => char.IsAsciiDigit(c))
        .ToList();

    public static readonly List<char> Lowercase = Enumerable.Range(0, 256)
        .Select(i => (char)i)
        .Where(c => char.IsAsciiLetterLower(c))
        .ToList();

    public static readonly List<char> Uppercase = Enumerable.Range(0, 256)
        .Select(i => (char)i)
        .Where(c => char.IsAsciiLetterUpper(c))
        .ToList();

    public static readonly List<char> HexLower = Enumerable.Range(0, 256)
        .Select(i => (char)i)
        .Where(c => char.IsAsciiHexDigitLower(c))
        .Concat(Numbers)
        .ToList();

    public static readonly List<char> PatChars = Numbers.Concat(Lowercase).ToList();

    public static string RandomChars(List<char> chars, int count, int seed = 0)
    {
        var r = new Random(seed);
        var result = new char[count];
        for (var i = 0; i < count; i++)
        {
            result[i] = chars[r.Next(chars.Count)];
        }
        return new string(result);
    }

    public static string RandomPAT(int seed = 0) => RandomChars(PatChars, PatLength, seed);

    [TestMethod]
    public void MatchPAT()
    {
        var pat = RandomPAT();
        Assert.AreEqual(0, PatScan.call_simd(pat), $"pat: {pat}");
    }

    [TestMethod]
    public void NonPAT()
    {
        var input = RandomChars(Numbers, 10 * PatLength);
        Assert.AreEqual(long.MinValue, PatScan.call_simd(input), $"input: {input}");
    }

    [TestMethod]
    public void NonPAT_Long()
    {
        var input = RandomChars(Numbers, 100_000);
        Assert.AreEqual(long.MinValue, PatScan.call_simd(input), $"input: {input}");
    }

    [TestMethod]
    public void Hex_Long()
    {
        var input = RandomChars(HexLower, 100_000);
        Assert.AreEqual(long.MinValue, PatScan.call_simd(input), $"input: {input}");
    }

    [TestMethod]
    public void Pat_After_Hex_Long()
    {
        var input = RandomChars(HexLower, 100_000) + RandomPAT() + RandomChars(HexLower, 1_000);
        Assert.AreEqual(100_000, PatScan.call_simd(input), delta: PatLength, message: $"input: {input}");
    }
}