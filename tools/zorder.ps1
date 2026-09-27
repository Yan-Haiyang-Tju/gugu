Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class ZO {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtrW(IntPtr h, int idx);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@
$target = (Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1).Id
if (-not $target) { Write-Output "gugu not running"; exit 1 }

$script:idx = 0
$script:out = New-Object System.Collections.ArrayList
$cb = [ZO+EnumProc]{
  param($h, $l)
  $script:idx++
  if ($script:idx -le 12 -or ([ZO]::GetWindowThreadProcessId($h, [ref]$null) -ne $null)) { }
  $p = 0
  [void][ZO]::GetWindowThreadProcessId($h, [ref]$p)
  $s = New-Object System.Text.StringBuilder 256
  [void][ZO]::GetClassNameW($h, $s, 256)
  $ex = [ZO]::GetWindowLongPtrW($h, -20).ToInt64()
  $tm = ($ex -band 0x8) -ne 0
  $isOurs = ($p -eq $target)
  # print the first 10 in z-order, plus anything of ours
  if ($script:idx -le 10 -or $isOurs) {
    $tag = if ($isOurs) { "  <== OURS" } else { "" }
    [void]$script:out.Add(("#{0,-3} class={1,-26} pid={2,-7} visible={3,-5} topmost={4}{5}" -f $script:idx, $s.ToString(), $p, [ZO]::IsWindowVisible($h), $tm, $tag))
  }
  return $true
}
[void][ZO]::EnumWindows($cb, [IntPtr]::Zero)
$script:out | ForEach-Object { Write-Output $_ }
Write-Output ("(顶层窗口总数 " + $script:idx + "，顺序 = z-order 自上而下)")
