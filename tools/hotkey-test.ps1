Add-Type @"
using System;
using System.Runtime.InteropServices;
public class HK {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
}
"@
# VK_LCONTROL = 0xA2 ; KEYEVENTF_KEYUP = 0x0002 ; VK_ESCAPE = 0x1B
$action = $env:HK_ACTION
if ($action -eq "esc") {
  [HK]::keybd_event(0x1B, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [HK]::keybd_event(0x1B, 0, 2, [UIntPtr]::Zero)
  Write-Output "sent ESC"
} else {
  # two lone Ctrl taps, 200ms apart (within the 420ms double-tap window)
  [HK]::keybd_event(0xA2, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [HK]::keybd_event(0xA2, 0, 2, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 200
  [HK]::keybd_event(0xA2, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [HK]::keybd_event(0xA2, 0, 2, [UIntPtr]::Zero)
  Write-Output "sent double-ctrl"
}
