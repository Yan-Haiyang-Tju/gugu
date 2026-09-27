Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class PM {
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@
function C([IntPtr]$h) { $s = New-Object System.Text.StringBuilder 256; [void][PM]::GetClassNameW($h, $s, 256); $s.ToString() }

$shell = [PM]::GetShellWindow()
Write-Output ("shell = " + $shell + " class=" + (C $shell))

$script:kids = New-Object System.Collections.ArrayList
$cb = [PM+EnumProc]{ param($h, $l) [void]$script:kids.Add($h); return $true }
[void][PM]::EnumChildWindows($shell, $cb, [IntPtr]::Zero)
Write-Output ("descendants of Progman: " + $script:kids.Count)
foreach ($k in $script:kids) {
  $r = New-Object PM+RECT
  [void][PM]::GetWindowRect($k, [ref]$r)
  Write-Output ("  {0,-22} hwnd={1,-9} parent={2,-9} visible={3} rect=({4},{5})-({6},{7})" -f (C $k), $k, [PM]::GetParent($k), [PM]::IsWindowVisible($k), $r.Left, $r.Top, $r.Right, $r.Bottom)
}
