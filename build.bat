@echo off
chcp 65001 >nul
powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1"
pause
