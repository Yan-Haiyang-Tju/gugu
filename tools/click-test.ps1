Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CT {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
# Must run before any DPI-dependent call so coordinates are true physical pixels.
[void][CT]::SetProcessDpiAwarenessContext([IntPtr](-4))

function C([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "-" }; $s = New-Object System.Text.StringBuilder 256; [void][CT]::GetClassNameW($h, $s, 256); $s.ToString() }
function PidOf([IntPtr]$h) { $p = 0; [void][CT]::GetWindowThreadProcessId($h, [ref]$p); $p }

$x = [int]$env:CLICK_X
$y = [int]$env:CLICK_Y
[void][CT]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 150
$p = New-Object CT+POINT
[void][CT]::GetCursorPos([ref]$p)
Write-Output ("cursor requested=($x,$y) actual=(" + $p.X + "," + $p.Y + ")")

$hit = [CT]::WindowFromPoint($p)
Write-Output ("hit class=" + (C $hit) + " pid=" + (PidOf $hit))

if ($env:CLICK_GO -eq "1") {
  # MOUSEEVENTF_LEFTDOWN = 0x0002, MOUSEEVENTF_LEFTUP = 0x0004
  # (1 = MOUSEEVENTF_MOVE, easy to mistake - sending 1/2 never produces a real click)
  [CT]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)   # release any stuck button first
  Start-Sleep -Milliseconds 40
  [CT]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)   # left down
  Start-Sleep -Milliseconds 70
  [CT]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)   # left up
  Write-Output "clicked (down+up)"
} else {
  Write-Output "(dry run, set CLICK_GO=1 to actually click)"
}
