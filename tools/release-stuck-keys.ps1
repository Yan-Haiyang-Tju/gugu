Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RK {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
}
"@
# Key-up for every virtual key: harmless for keys that are not held, and it
# guarantees no synthetic key is left stuck down (e.g. Win held by an interrupted test).
$UP = 2
for ($vk = 1; $vk -le 254; $vk++) {
  [RK]::keybd_event([byte]$vk, 0, $UP, [UIntPtr]::Zero)
}
# Mouse buttons too: 0x0004 = LEF TUP, 0x0010 = RIGHTUP, 0x0040 = MIDDLEUP
[RK]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
[RK]::mouse_event(16, 0, 0, 0, [UIntPtr]::Zero)
[RK]::mouse_event(64, 0, 0, 0, [UIntPtr]::Zero)

Start-Sleep -Milliseconds 300
$still = @()
foreach ($name in @{ 'Win-L' = 0x5B; 'Win-R' = 0x5C; 'Shift' = 0x10; 'Ctrl' = 0x11; 'Alt' = 0x12; 'Caps' = 0x14 }.GetEnumerator()) {
  $state = [RK]::GetAsyncKeyState($name.Value)
  if ($state -band 0x8000) { $still += $name.Key }
}
if ($still.Count -gt 0) {
  Write-Output ("STILL DOWN: " + ($still -join ", "))
} else {
  Write-Output "all modifier keys released"
}
