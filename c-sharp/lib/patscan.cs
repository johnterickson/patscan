using System.Runtime.InteropServices;

namespace patscan;

public static class PatScan {
    [DllImport("patscan_lib.dll", CharSet = CharSet.Unicode)]
    private unsafe static extern UInt32 simd_c([MarshalAs(UnmanagedType.LPUTF8Str)] string str, UInt32 strLen);

    public static long call_simd(string str)
    {
        UInt32 result = simd_c(str, (UInt32)str.Length);
        if (result == UInt32.MaxValue)
        {
            return long.MinValue;
        }
        return (long)result;
    }

}
