# GUGU 环境脚本 v2：卸载 C 盘半成品实例 → 重装到 D:\BuildTools
# 修正：setup.exe 不支持 --wait（其本身即同步阻塞），改加 --force 清理残留
$log = 'D:\coding_space\project01_calendar\calender\tools\vs-reinstall.log'
"=== start v2 $(Get-Date) ===" | Out-File $log

$setup = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe'
$oldPath = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools'

if (Test-Path $oldPath) {
  "--- uninstall C-drive partial instance ---" | Out-File $log -Append
  & $setup uninstall --installPath $oldPath --quiet --norestart --force 2>&1 | Out-File $log -Append
  "uninstall exit: $LASTEXITCODE" | Out-File $log -Append
} else {
  "no C-drive instance found, skip uninstall" | Out-File $log -Append
}

"--- install to D:\BuildTools ---" | Out-File $log -Append
& $setup install `
  --productId Microsoft.VisualStudio.Product.BuildTools `
  --channelId VisualStudio.17.Release `
  --channelUri https://aka.ms/vs/17/release/channel `
  --installPath D:\BuildTools `
  --add Microsoft.VisualStudio.Workload.VCTools `
  --includeRecommended `
  --nocache --quiet --norestart --force 2>&1 | Out-File $log -Append
"install exit: $LASTEXITCODE" | Out-File $log -Append
"=== done $(Get-Date) ===" | Out-File $log -Append
