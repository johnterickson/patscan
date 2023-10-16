using System.Runtime.InteropServices;

namespace patscan;

public static class PatScan {
    [DllImport("patscan_rs.dll", EntryPoint = "simd_c")]
    private unsafe static extern uint simd_c_windows(char* str, uint strLen);
    [DllImport("libpatscan_rs.so", EntryPoint = "simd_c")]
    private unsafe static extern uint simd_c_linux(char* str, uint strLen);

    public unsafe static long call_simd(string str)
    {
        uint result;
        fixed (char* p = str)
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                result = simd_c_windows(p, (uint)str.Length);
            }
            else
            {
                result = simd_c_linux(p, (uint)str.Length);
            }
        }
        if (result == uint.MaxValue)
        {
            return long.MinValue;
        }
        return result;
    }

}
