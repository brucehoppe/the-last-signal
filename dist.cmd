@echo off
rem Build a Windows distribution on a Windows machine:
rem   dist\the-last-signal-windows-x64.zip
rem (On macOS, scripts/dist.sh builds both the macOS and Windows packages.)
setlocal
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed or is not on PATH. Read START-HERE.md.
  pause
  exit /b 1
)
cargo build --locked --release
if errorlevel 1 goto failed
set STAGE=dist\the-last-signal-windows-x64
if exist "%STAGE%" rmdir /s /q "%STAGE%"
if exist "%STAGE%.zip" del "%STAGE%.zip"
mkdir "%STAGE%"
copy /y target\release\the-last-signal.exe "%STAGE%\" >nul
for %%F in (README.md START-HERE.md LICENSE config.example.json assets\fonts\OFL.txt) do copy /y "%%F" "%STAGE%\" >nul
powershell -NoProfile -Command "Compress-Archive -Path '%STAGE%' -DestinationPath '%STAGE%.zip' -Force"
if errorlevel 1 goto failed
echo Built %STAGE%.zip
pause
exit /b 0
:failed
echo The distribution build failed. Read the output above.
pause
exit /b 1
