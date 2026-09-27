Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class FW {
  [DllImport("user32.dll")] public static extern IntPtr GetDesktopWindow();
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int idx);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@
function C([IntPtr]$h) { $s = New-Object System.Text.StringBuilder 256; [void][FW]::GetClassNameW($h, $s, 256); $s.ToString() }

$target = (Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1).Id
if (-not $target) { Write-Output "gugu.exe not running"; exit 1 }
Write-Output ("gugu pid = " + $target)

$script:hits = New-Object System.Collections.ArrayList
$cb = [FW+EnumProc]{
  param($h, $l)
  $p = 0
  [void][FW]::GetWindowThreadProcessId($h, [ref]$p)
  if ($p -eq $target) {
    $r = New-Object FW+RECT
    [void][FW]::GetWindowRect($h, [ref]$r)
    $style = [FW]::GetWindowLongPtrW($h, -16).ToInt64()
    $isChild = ($style -band 0x40000000) -ne 0
    $chain = @()
    $cur = $h
    for ($i = 0; $i -lt 6 -and $cur -ne [IntPtr]::Zero; $i++) {
      $chain += ((C $cur) + "#" + $cur)
      $cur = [FW]::GetParent($cur)
    }
    [void]$script:hits.Add(("class={0} hwnd={1} WS_CHILD={2} visible={3} rect={4}x{5}" -f (C $h), $h, $isChild, [FW]::IsWindowVisible($h), ($r.Right-$r.Left), ($r.Bottom-$r.Top)))
    [void]$script:hits.Add(("     ancestors: " + ($chain -join " -> ")))
  }
  return $true
}
[void][FW]::EnumChildWindows([FW]::GetDesktopWindow(), $cb, [IntPtr]::Zero)
$script:hits | ForEach-Object { Write-Output $_ }
