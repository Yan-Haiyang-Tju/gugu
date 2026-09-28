Add-Type @"
using System;
using System.Runtime.InteropServices;
public class HK {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
}
"@
# VK_LCONTROL = 0xA2 ; VK_ESCAPE = 0x1B ; KEYEVENTF_KEYUP = 0x0002
$VK_CTRL = 0xA2
$VK_ESC = 0x1B
$UP = 2
# keybd_event with a zero scan code makes Windows synthesise a bogus vk=0 event,
# which the app correctly treats as "another key was pressed". Always pass a real one.
function Scan([byte]$vk) { return [byte]([HK]::MapVirtualKey($vk, 0)) }
function Tap([byte]$vk) {
  [HK]::keybd_event($vk, (Scan $vk), 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [HK]::keybd_event($vk, (Scan $vk), $UP, [UIntPtr]::Zero)
}

# Every key pressed here is released in the finally block: an interrupted script that
# leaves a modifier held down makes the whole keyboard behave as if it were stuck.
try {
  switch ($env:HK_ACTION) {
    "esc" {
      Tap $VK_ESC
      Write-Output "sent ESC"
    }
    "quad" {
      # 4 taps 200ms apart = two double-tap gestures back to back.
      # The second gesture triggers ~360ms after the first - this is the
      # "open, then immediately press again to close" case that a too-long
      # de-duplication cooldown used to swallow.
      for ($i = 0; $i -lt 4; $i++) {
        Tap $VK_CTRL
        if ($i -lt 3) { Start-Sleep -Milliseconds 160 }
      }
      Write-Output "sent quad-ctrl (two gestures)"
    }
    default {
      # two lone Ctrl taps, 200ms apart (within the 420ms double-tap window)
      Tap $VK_CTRL
      Start-Sleep -Milliseconds 200
      Tap $VK_CTRL
      Write-Output "sent double-ctrl"
    }
  }
} finally {
  [HK]::keybd_event($VK_CTRL, (Scan $VK_CTRL), $UP, [UIntPtr]::Zero)
  [HK]::keybd_event($VK_ESC, (Scan $VK_ESC), $UP, [UIntPtr]::Zero)
}
