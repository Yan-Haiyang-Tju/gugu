Add-Type @"
using System;
using System.Runtime.InteropServices;
public class SK {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder s, int m);
}
"@
$vk = [byte]$env:SK_VK
if ($vk -eq 0) { $vk = 0x41 }   # 'A'
$h = [SK]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[void][SK]::GetClassNameW($h, $sb, 256)
Write-Output ("foreground=" + $sb.ToString() + "  sending vk=" + $vk)
[SK]::keybd_event($vk, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 40
[SK]::keybd_event($vk, 0, 2, [UIntPtr]::Zero)
Write-Output "sent"
