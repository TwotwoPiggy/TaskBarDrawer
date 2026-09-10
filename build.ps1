# Stop running instance if any
Get-Process -Name "taskbar-drawer" -ErrorAction SilentlyContinue | Stop-Process -Force

# Build in release mode
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    exit $LASTEXITCODE
}

# Copy to root
Copy-Item "target\release\taskbar-drawer.exe" -Destination "taskbar-drawer.exe" -Force

# Copy to V:\软件\2.Release
$targetDir = "V:\软件\2.Release"
if (Test-Path $targetDir) {
    Copy-Item "target\release\taskbar-drawer.exe" -Destination "$targetDir\taskbar-drawer.exe" -Force
    Write-Host "✅ 已成功输出并同步至: $targetDir\taskbar-drawer.exe" -ForegroundColor Green
} else {
    Write-Host "⚠️ 未检测到目标目录: $targetDir" -ForegroundColor Yellow
}

Write-Host "🎉 发布打包完成！" -ForegroundColor Cyan
