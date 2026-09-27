Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$out = $env:SHOT_OUT
if (-not $out) { $out = "D:\tmp\gugu-shot.png" }
$dir = Split-Path -Parent $out
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$v = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap($v.Width, $v.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($v.Left, $v.Top, 0, 0, $bmp.Size)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Output ("saved: " + $out + " " + $v.Width + "x" + $v.Height)
