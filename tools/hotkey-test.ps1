Add-Type @"
using System;
using System.Runtime.InteropServices;
public class HK {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
}
"@
# VK_LCONTROL = 0xA2 ; VK_ESCAPE = 0x1B ; KEYEVENTF_KEYUP = 0x0002
$VK_CTRL = 0xA2
$VK_ESC = 0x1B
$UP = 2

# IMPORTANT: every injected key is released in the finally block.
# An interrupted script that left a modifier held down makes the whole
# keyboard behave like that modifier is stuck (e.g. a held Win key turns
# "A" into Win+A). tools/release-stuck-keys.ps1 is the emergency fix.
try {
  if ($env:HK_ACTION -eq "esc") {
    [HK]::keybd_event($VK_ESC, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [HK]::keybd_event($VK_ESC, 0, $UP, [UIntPtr]::Zero)
    Write-Output "sent ESC"
  } else {
    # two lone Ctrl taps, 200ms apart (within the 420ms double-tap window)
    [HK]::keybd_event($VK_CTRL, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [HK]::keybd_event($VK_CTRL, 0, $UP, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    [HK]::keybd_event($VK_CTRL, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [HK]::keybd_event($VK_CTRL, 0, $UP, [UIntPtr]::Zero)
    Write-Output "sent double-ctrl"
  }
} finally {
  [HK]::keybd_event($VK_CTRL, 0, $UP, [UIntPtr]::Zero)
  [HK]::keybd_event($VK_ESC, 0, $UP, [UIntPtr]::Zero)
}
