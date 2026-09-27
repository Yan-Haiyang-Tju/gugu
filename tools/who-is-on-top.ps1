Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WOT {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int idx);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[void][WOT]::SetProcessDpiAwarenessContext([IntPtr](-4))

function Cls([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "-" }; $s = New-Object System.Text.StringBuilder 256; [void][WOT]::GetClassNameW($h, $s, 256); $s.ToString() }
function Ttl([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "" }; $s = New-Object System.Text.StringBuilder 256; [void][WOT]::GetWindowTextW($h, $s, 256); $s.ToString() }
function PidOf([IntPtr]$h) { $p = 0; [void][WOT]::GetWindowThreadProcessId($h, [ref]$p); $p }

$pt = New-Object WOT+POINT
$pt.X = [int]$env:PT_X
$pt.Y = [int]$env:PT_Y
$h = [WOT]::WindowFromPoint($pt)
Write-Output ("point ($($pt.X),$($pt.Y)) -> class=" + (Cls $h) + " pid=" + (PidOf $h) + " title='" + (Ttl $h) + "'")

# walk up to the top-level window and report whether it is TOPMOST
$top = [WOT]::GetAncestor($h, 2)  # GA_ROOT
if ($top -ne [IntPtr]::Zero) {
  $ex = [WOT]::GetWindowLongPtrW($top, -20).ToInt64()
  $topmost = ($ex -band 0x8) -ne 0
  $proc = (Get-Process -Id (PidOf $top) -ErrorAction SilentlyContinue).ProcessName
  Write-Output ("  root: class=" + (Cls $top) + " pid=" + (PidOf $top) + " proc=" + $proc + " TOPMOST=" + $topmost)
}
