using System.Runtime.InteropServices;

namespace patscan;

public static class PatScan {
    [DllImport("patscan_lib.dll", CharSet = CharSet.Unicode)]
    private unsafe static extern uint simd_c(char* str, uint strLen);

    public unsafe static long call_simd(string str)
    {
        uint result;
        fixed (char* p = str)
        {
            result = simd_c(p, (uint)str.Length);
        }
        if (result == uint.MaxValue)
        {
            return long.MinValue;
        }
        return result;
    }

}
