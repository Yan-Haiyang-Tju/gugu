Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class DT2 {
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int m);
}
"@
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DT3 {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr ctx);
}
"@
[void][DT3]::SetProcessDpiAwarenessContext([IntPtr](-4))

function FgName {
  $h = [DT2]::GetForegroundWindow()
  $sb = New-Object System.Text.StringBuilder 256
  [void][DT2]::GetClassNameW($h, $sb, 256)
  return $sb.ToString()
}

# 1) show desktop, 2) click an empty spot so Progman becomes the foreground
[DT2]::keybd_event(0x5B, 0, 0, [UIntPtr]::Zero)
[DT2]::keybd_event(0x44, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 40
[DT2]::keybd_event(0x44, 0, 2, [UIntPtr]::Zero)
[DT2]::keybd_event(0x5B, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 900

[void][DT3]::SetCursorPos([int]$env:CLICK_X3, [int]$env:CLICK_Y3)
Start-Sleep -Milliseconds 150
[DT3]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 60
[DT3]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 700
Write-Output ("foreground after desktop click = " + (FgName))

# 3) trigger show_panel through the single-instance path (no keyboard hook involved)
Start-Process -FilePath $env:GUGU_EXE -WindowStyle Hidden
Start-Sleep -Seconds 5
Write-Output ("foreground after trigger = " + (FgName))
