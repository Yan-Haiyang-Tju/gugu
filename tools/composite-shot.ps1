Add-Type -AssemblyName System.Drawing
# Composite a window capture (which has a transparent glass background) onto a
# neutral backdrop, so the screenshots read the way the UI does on screen
# without exposing the user's actual desktop.
$in = $env:SHOT_IN
$out = $env:SHOT_OUT
if (-not $in -or -not $out) { Write-Output "set SHOT_IN / SHOT_OUT"; exit 1 }

$src = [System.Drawing.Image]::FromFile($in)
$w = $src.Width
$h = $src.Height
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic

# soft warm-paper gradient, matching the app's own light theme
$rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
$c1 = [System.Drawing.ColorTranslator]::FromHtml("#F4F3F0")
$c2 = [System.Drawing.ColorTranslator]::FromHtml("#E6E5E1")
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $c1, $c2, 90.0)
$g.FillRectangle($brush, $rect)

$g.DrawImage($src, 0, 0, $w, $h)
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$brush.Dispose(); $g.Dispose(); $bmp.Dispose(); $src.Dispose()
Write-Output ("composited -> $out  ${w}x${h}")
