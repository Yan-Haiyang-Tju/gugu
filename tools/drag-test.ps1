Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DG {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[void][DG]::SetProcessDpiAwarenessContext([IntPtr](-4))

$h = [IntPtr][int]$env:DRAG_HWND
if ($h -eq [IntPtr]::Zero) { Write-Output "set DRAG_HWND"; exit 1 }
$r = New-Object DG+RECT
[void][DG]::GetWindowRect($h, [ref]$r)
Write-Output ("before: ({0},{1})-({2},{3})" -f $r.Left, $r.Top, $r.Right, $r.Bottom)

# grab point: fraction of the window, overridable from the caller
$rx = if ($env:DRAG_RX) { [double]$env:DRAG_RX } else { 0.45 }
$ry = if ($env:DRAG_RY) { [double]$env:DRAG_RY } else { 0.045 }
$sx = $r.Left + [int](($r.Right - $r.Left) * $rx)
$sy = $r.Top + [int](($r.Bottom - $r.Top) * $ry)
$dx = [int]$env:DRAG_DX
$dy = [int]$env:DRAG_DY
if ($dx -eq 0) { $dx = 140 }
if ($dy -eq 0) { $dy = 90 }

[void][DG]::SetCursorPos($sx, $sy)
Start-Sleep -Milliseconds 200
[void][DG]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)   # release any stuck button
Start-Sleep -Milliseconds 60
[void][DG]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)   # left down
Start-Sleep -Milliseconds 80
for ($i = 1; $i -le 12; $i++) {
  [void][DG]::SetCursorPos($sx + [int]($dx * $i / 12), $sy + [int]($dy * $i / 12))
  Start-Sleep -Milliseconds 25
}
Start-Sleep -Milliseconds 80
[void][DG]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)   # left up
Start-Sleep -Milliseconds 400

$r2 = New-Object DG+RECT
[void][DG]::GetWindowRect($h, [ref]$r2)
Write-Output ("after:  ({0},{1})-({2},{3})" -f $r2.Left, $r2.Top, $r2.Right, $r2.Bottom)
Write-Output ("moved:  dx={0} dy={1}" -f ($r2.Left - $r.Left), ($r2.Top - $r.Top))
