Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CapOne {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$h = [IntPtr][int]$env:CAP_HWND
if ($h -eq [IntPtr]::Zero) { Write-Output "set CAP_HWND"; exit 1 }
$r = New-Object CapOne+RECT
[void][CapOne]::GetWindowRect($h, [ref]$r)
# PowerShell is DPI-unaware, so GetWindowRect reports logical pixels.
# Scale up by the window's own DPI or the bitmap clips the right/bottom.
$dpi = [CapOne]::GetDpiForWindow($h)
if ($dpi -eq 0) { $dpi = 96 }
$scale = $dpi / 96.0
$w = [int](($r.Right - $r.Left) * $scale)
$ht = [int](($r.Bottom - $r.Top) * $scale)
if ($w -le 0 -or $ht -le 0) { Write-Output "bad rect"; exit 1 }
$out = $env:CAP_OUT
if (-not $out) { $out = "D:\tmp\gugu-widget.png" }
$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [CapOne]::PrintWindow($h, $hdc, 2)
$g.ReleaseHdc($hdc)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output ("saved $out  ${w}x${ht}  dpi=$dpi  PrintWindow=$ok")
