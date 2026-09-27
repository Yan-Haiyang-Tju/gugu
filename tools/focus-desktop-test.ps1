Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class FD2 {
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
}
"@
function Cls([IntPtr]$h) { if ($h -eq [IntPtr]::Zero) { return "-" }; $s = New-Object System.Text.StringBuilder 256; [void][FD2]::GetClassNameW($h, $s, 256); $s.ToString() }

$shell = [FD2]::GetShellWindow()
Write-Output ("shell(Progman) = " + $shell + " class=" + (Cls $shell))
[void][FD2]::SetForegroundWindow($shell)
Start-Sleep -Milliseconds 600
$fg = [FD2]::GetForegroundWindow()
Write-Output ("foreground now = " + $fg + " class=" + (Cls $fg))

# double-ctrl (two lone Ctrl taps 200ms apart)
[FD2]::keybd_event(0xA2, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 40
[FD2]::keybd_event(0xA2, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 200
[FD2]::keybd_event(0xA2, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 40
[FD2]::keybd_event(0xA2, 0, 2, [UIntPtr]::Zero)
Write-Output "sent double-ctrl with desktop focused"
