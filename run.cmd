@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed or is not on PATH. Read START-HERE.md.
  pause
  exit /b 1
)
cargo run --locked --release
if errorlevel 1 (
  echo Build or launch failed. Read the error above and START-HERE.md.
  pause
  exit /b 1
)
