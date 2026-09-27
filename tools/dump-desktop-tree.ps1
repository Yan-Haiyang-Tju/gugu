Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class DeskTree {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string win);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowExW(IntPtr parent, IntPtr after, string cls, string win);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int max);
  [DllImport("user32.dll")] public static extern IntPtr SendMessageTimeoutW(IntPtr h, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
"@

function ClassOf([IntPtr]$h) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][DeskTree]::GetClassNameW($h, $sb, 256)
  return $sb.ToString()
}

function TitleOf([IntPtr]$h) {
  $sb = New-Object System.Text.StringBuilder 256
  [void][DeskTree]::GetWindowTextW($h, $sb, 256)
  return $sb.ToString()
}

function Dump([string]$label) {
  Write-Output "===== $label ====="
  $progman = [DeskTree]::FindWindowW("Progman", $null)
  Write-Output ("Progman hwnd = " + $progman)
  if ($progman -ne [IntPtr]::Zero) {
    DumpChildren $progman "  (child of Progman)"
  }
  foreach ($h in (GetTopLevelWorkerW)) {
    Write-Output ("top-level WorkerW hwnd=" + $h + " title='" + (TitleOf $h) + "'")
    DumpChildren $h "  (child of that WorkerW)"
  }
}

function DumpChildren([IntPtr]$parent, [string]$indent) {
  $script:kids = New-Object System.Collections.ArrayList
  $cb = [DeskTree+EnumProc]{
    param($h, $l)
    [void]$script:kids.Add($h)
    return $true
  }
  [void][DeskTree]::EnumChildWindows($parent, $cb, [IntPtr]::Zero)
  foreach ($k in $script:kids) {
    $c = ClassOf $k
    if ($c -eq "WorkerW" -or $c -eq "SHELLDLL_DefView" -or $c -eq "SysListView32" -or $c -eq "Progman") {
      Write-Output ($indent + " -> " + $c + " hwnd=" + $k)
    }
  }
}

function GetTopLevelWorkerW() {
  $script:tops = New-Object System.Collections.ArrayList
  $cb = [DeskTree+EnumProc]{
    param($h, $l)
    if ((ClassOf $h) -eq "WorkerW") { [void]$script:tops.Add($h) }
    return $true
  }
  [void][DeskTree]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:tops
}

Dump "BEFORE 0x052C"
$progman = [DeskTree]::FindWindowW("Progman", $null)
if ($progman -ne [IntPtr]::Zero) {
  $r = [IntPtr]::Zero
  [void][DeskTree]::SendMessageTimeoutW($progman, 0x052C, [IntPtr]::Zero, [IntPtr]::Zero, 0, 1000, [ref]$r)
  Start-Sleep -Milliseconds 500
}
Dump "AFTER 0x052C"
