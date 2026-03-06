@echo off
echo Building monitor-daemon...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Build failed.
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo Starting monitor-daemon...
.\target\release\monitor-daemon.exe

echo.
echo Application exited with code %ERRORLEVEL%
pause
