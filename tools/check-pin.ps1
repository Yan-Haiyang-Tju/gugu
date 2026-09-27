Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CP {
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int idx);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@
function C([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "-" }; $s = New-Object System.Text.StringBuilder 256; [void][CP]::GetClassNameW($h, $s, 256); $s.ToString() }

$target = (Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1).Id
if (-not $target) { Write-Output "gugu.exe not running"; exit 1 }

$shell = [CP]::GetShellWindow()
Write-Output ("Progman = " + $shell)

# the WorkerW child of Progman = wallpaper layer
$script:w = [IntPtr]::Zero
$cb = [CP+EnumProc]{
  param($h, $l)
  if ((C $h) -eq "WorkerW" -and [CP]::GetParent($h) -eq $shell) { $script:w = $h }
  return $true
}
[void][CP]::EnumChildWindows($shell, $cb, [IntPtr]::Zero)
Write-Output ("wallpaper WorkerW = " + $script:w)

# direct children of that WorkerW
Write-Output "--- direct children of the wallpaper WorkerW ---"
$script:kids = New-Object System.Collections.ArrayList
$cb2 = [CP+EnumProc]{
  param($h, $l)
  $p = 0
  [void][CP]::GetWindowThreadProcessId($h, [ref]$p)
  if ([CP]::GetParent($h) -eq $script:w) {
    [void]$script:kids.Add(("  class={0} hwnd={1} pid={2} visible={3}" -f (C $h), $h, $p, [CP]::IsWindowVisible($h)))
  }
  return $true
}
[void][CP]::EnumChildWindows($script:w, $cb2, [IntPtr]::Zero)
$script:kids | ForEach-Object { Write-Output $_ }

# where is our widget?
Write-Output "--- gugu Tauri windows (top level) ---"
$script:tops = New-Object System.Collections.ArrayList
$cb3 = [CP+EnumProc]{
  param($h, $l)
  $p = 0
  [void][CP]::GetWindowThreadProcessId($h, [ref]$p)
  if ($p -eq $target -and (C $h) -eq "Tauri Window") {
    [void]$script:tops.Add(("  hwnd={0} parent={1} ancestorParent={2} visible={3}" -f $h, [CP]::GetParent($h), [CP]::GetAncestor($h, 1), [CP]::IsWindowVisible($h)))
  }
  return $true
}
[void][CP]::EnumWindows($cb3, [IntPtr]::Zero)
if ($script:tops.Count -eq 0) { Write-Output "  (none at top level -> widget is a child window, i.e. pinned)" }
$script:tops | ForEach-Object { Write-Output $_ }
