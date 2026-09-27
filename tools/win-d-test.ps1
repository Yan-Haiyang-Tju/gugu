Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WD {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
"@
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WD2 {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
}
"@
[void][WD2]::SetProcessDpiAwarenessContext([IntPtr](-4))
$VK_LWIN = 0x5B
$VK_D = 0x44
$VK_CTRL = 0xA2
$UP = 2

function Tap([byte]$vk) {
  [WD]::keybd_event($vk, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 30
  [WD]::keybd_event($vk, 0, $UP, [UIntPtr]::Zero)
}

$mode = $env:WD_MODE
if ($mode -eq "clickdesktop") {
  # Win+D first, then a real click on an empty desktop spot so the foreground
  # becomes Progman (that is the actual "on the desktop" state, unlike Win+D alone).
  [WD]::keybd_event($VK_LWIN, 0, 0, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_D, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [WD]::keybd_event($VK_D, 0, $UP, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_LWIN, 0, $UP, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 900
  [WD2]::SetCursorPos([int]$env:CLICK_X2, [int]$env:CLICK_Y2)
  Start-Sleep -Milliseconds 150
  [WD2]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [WD2]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 600
  $fg = [WD]::GetForegroundWindow()
  $shell = [WD]::GetShellWindow()
  $p = 0; [void][WD]::GetWindowThreadProcessId($fg, [ref]$p)
  Write-Output ("after click on desktop: fg=" + $fg + " shell=" + $shell + " isShell=" + ($fg -eq $shell) + " fgProc=" + (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName)
  Tap $VK_CTRL
  Start-Sleep -Milliseconds 200
  Tap $VK_CTRL
  Write-Output "sent double-ctrl while Progman focused"
} elseif ($mode -eq "desktop") {
  # Win+D : show desktop (and give it focus)
  [WD]::keybd_event($VK_LWIN, 0, 0, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_D, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [WD]::keybd_event($VK_D, 0, $UP, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_LWIN, 0, $UP, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 900
  $fg = [WD]::GetForegroundWindow()
  $shell = [WD]::GetShellWindow()
  $p = 0; [void][WD]::GetWindowThreadProcessId($fg, [ref]$p)
  Write-Output ("after Win+D: fg=" + $fg + " shell=" + $shell + " sameAsShell=" + ($fg -eq $shell) + " fgProc=" + (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName)
  # double ctrl with the desktop focused
  Tap $VK_CTRL
  Start-Sleep -Milliseconds 200
  Tap $VK_CTRL
  Write-Output "sent double-ctrl while desktop focused"
} else {
  [WD]::keybd_event($VK_LWIN, 0, 0, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_D, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [WD]::keybd_event($VK_D, 0, $UP, [UIntPtr]::Zero)
  [WD]::keybd_event($VK_LWIN, 0, $UP, [UIntPtr]::Zero)
  Write-Output "sent Win+D (restore)"
}
