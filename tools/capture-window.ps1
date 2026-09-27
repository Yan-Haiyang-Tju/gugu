Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WinCap {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@

$target = (Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1).Id
if (-not $target) { Write-Output "gugu.exe not running"; exit 1 }

$script:found = New-Object System.Collections.ArrayList
$cb = [WinCap+EnumProc]{
  param($h, $l)
  $p = 0
  [void][WinCap]::GetWindowThreadProcessId($h, [ref]$p)
  if ($p -eq $target) {
    $cls = New-Object System.Text.StringBuilder 256
    [void][WinCap]::GetClassNameW($h, $cls, 256)
    if ($cls.ToString() -eq "Tauri Window") {
      $r = New-Object WinCap+RECT
      [void][WinCap]::GetWindowRect($h, [ref]$r)
      [void]$script:found.Add(@{ h = $h; w = ($r.Right - $r.Left); ht = ($r.Bottom - $r.Top); vis = [WinCap]::IsWindowVisible($h) })
    }
  }
  return $true
}
[void][WinCap]::EnumWindows($cb, [IntPtr]::Zero)

Write-Output ("Tauri windows: " + $script:found.Count)
$i = 0
foreach ($w in $script:found) {
  $i++
  if ($w.w -le 0 -or $w.ht -le 0) { continue }
  $bmp = New-Object System.Drawing.Bitmap($w.w, $w.ht)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  # flags=2 (PW_RENDERFULLCONTENT) is needed for DirectComposition/webview surfaces
  $ok = [WinCap]::PrintWindow($w.h, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $out = "D:\tmp\gugu-win-$i.png"
  $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ("  #$i hwnd=$($w.h) visible=$($w.vis) size=$($w.w)x$($w.ht) PrintWindow=$ok -> $out")
}
