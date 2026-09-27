Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class FD {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@

function C([IntPtr]$h) { $s = New-Object System.Text.StringBuilder 256; [void][FD]::GetClassNameW($h, $s, 256); $s.ToString() }
function T([IntPtr]$h) { $s = New-Object System.Text.StringBuilder 256; [void][FD]::GetWindowTextW($h, $s, 256); $s.ToString() }

Write-Output ("GetShellWindow() = " + [FD]::GetShellWindow())
$sh = [FD]::GetShellWindow()
if ($sh -ne [IntPtr]::Zero) { Write-Output ("  shell window class = " + (C $sh) + " title='" + (T $sh) + "'") }

# Collect every top-level window, group by class.
$script:all = New-Object System.Collections.ArrayList
$cb = [FD+EnumProc]{ param($h, $l) [void]$script:all.Add($h); return $true }
[void][FD]::EnumWindows($cb, [IntPtr]::Zero)

Write-Output ("top-level window count = " + $script:all.Count)
$byClass = @{}
foreach ($h in $script:all) {
  $c = C $h
  if (-not $byClass.ContainsKey($c)) { $byClass[$c] = 0 }
  $byClass[$c]++
}
Write-Output "--- class histogram ---"
$byClass.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { Write-Output ("  {0,-40} {1}" -f $_.Key, $_.Value) }

# Find any window (top-level or nested) whose class is SHELLDLL_DefView, and report its ancestors.
Write-Output "--- searching SHELLDLL_DefView ---"
$script:defviews = New-Object System.Collections.ArrayList
$cb2 = [FD+EnumProc]{
  param($h, $l)
  if ((C $h) -eq "SHELLDLL_DefView") { [void]$script:defviews.Add($h) }
  return $true
}
foreach ($h in $script:all) { [void][FD]::EnumChildWindows($h, $cb2, [IntPtr]::Zero) }
Write-Output ("SHELLDLL_DefView found: " + $script:defviews.Count)
foreach ($d in $script:defviews) {
  $p = [FD]::GetParent($d)
  $pp = [FD]::GetParent($p)
  Write-Output ("  defview hwnd=" + $d + " parent=" + $p + " (" + (C $p) + ") grandparent=" + $pp + " (" + $(if ($pp -ne [IntPtr]::Zero) { C $pp } else { "-" }) + ")")
}
