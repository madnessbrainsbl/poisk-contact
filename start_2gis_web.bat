@echo off
cd /d "%~dp0"
chcp 65001 >nul
where parser-2gis-new >nul 2>nul
if errorlevel 1 (
    echo Install the optional Parser2GIS companion first. See README.md.
    pause
    exit /b 1
)
echo Parser2GIS dashboard: http://127.0.0.1:8666
echo Export CSV from the dashboard and import it in PoiskContact.
parser-2gis-new --web --web-port 8666
pause
