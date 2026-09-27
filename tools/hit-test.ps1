Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class HT {
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
function C([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "-" }; $s = New-Object System.Text.StringBuilder 256; [void][HT]::GetClassNameW($h, $s, 256); $s.ToString() }
function PidOf([IntPtr]$h) { $p = 0; [void][HT]::GetWindowThreadProcessId($h, [ref]$p); $p }

$gugu = Get-Process gugu -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $gugu) { Write-Output "gugu.exe not running"; exit 1 }

# locate widget: the Tauri window owned by gugu that is 300x560
$r = New-Object HT+RECT
$hwnd = [IntPtr]::Zero
# reuse the known geometry from the saved settings instead of enumerating: read from env
$wx = [int]$env:WIDGET_X
$wy = [int]$env:WIDGET_Y
if ($wx -eq 0 -and $wy -eq 0) { Write-Output "set WIDGET_X / WIDGET_Y env vars"; exit 1 }

Write-Output ("gugu pid = " + $gugu.Id + "  widget assumed at (" + $wx + "," + $wy + ")")

$probeX = $wx + 40
$probeY = $wy + 60
$pt = New-Object HT+POINT
$pt.X = $probeX
$pt.Y = $probeY
$hit = [HT]::WindowFromPoint($pt)
Write-Output ("WindowFromPoint(" + $probeX + "," + $probeY + ") -> hwnd=" + $hit + " class=" + (C $hit) + " pid=" + (PidOf $hit))
$root = [HT]::GetAncestor($hit, 2)
Write-Output ("  its root ancestor -> hwnd=" + $root + " class=" + (C $root) + " pid=" + (PidOf $root))

Write-Output "--- same point, but hit-testing each gugu window rect ---"
Get-Process gugu -ErrorAction SilentlyContinue | ForEach-Object { }
