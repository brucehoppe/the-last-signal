@echo off
setlocal
cd /d "%~dp0"
cargo fmt -- --check
if errorlevel 1 goto failed
cargo test --locked --all-targets
if errorlevel 1 goto failed
cargo clippy --locked --all-targets -- -D warnings
if errorlevel 1 goto failed
echo All development checks passed.
pause
exit /b 0
:failed
echo A check failed. Read the output above.
pause
exit /b 1
