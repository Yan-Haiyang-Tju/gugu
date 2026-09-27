Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WinDump {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@

$target = (Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1).Id
if (-not $target) { Write-Output "gugu.exe not running"; exit 1 }
Write-Output ("gugu.exe PID = " + $target)

# Output written inside a native callback is lost, so collect first and print after.
$script:lines = New-Object System.Collections.ArrayList
$cb = [WinDump+EnumProc]{
  param($h, $l)
  $p = 0
  [void][WinDump]::GetWindowThreadProcessId($h, [ref]$p)
  if ($p -eq $target) {
    $cls = New-Object System.Text.StringBuilder 256
    [void][WinDump]::GetClassNameW($h, $cls, 256)
    $txt = New-Object System.Text.StringBuilder 256
    [void][WinDump]::GetWindowTextW($h, $txt, 256)
    $r = New-Object WinDump+RECT
    [void][WinDump]::GetWindowRect($h, [ref]$r)
    [void]$script:lines.Add(("hwnd={0} class={1} visible={2} iconic={3} parent={4}" -f $h, $cls.ToString(), [WinDump]::IsWindowVisible($h), [WinDump]::IsIconic($h), [WinDump]::GetParent($h)))
    [void]$script:lines.Add(("    rect=({0},{1})-({2},{3}) size={4}x{5}" -f $r.Left, $r.Top, $r.Right, $r.Bottom, ($r.Right - $r.Left), ($r.Bottom - $r.Top)))
  }
  return $true
}
[void][WinDump]::EnumWindows($cb, [IntPtr]::Zero)
$script:lines | ForEach-Object { Write-Output $_ }
Write-Output ("total top-level windows for gugu: " + $script:lines.Count)
