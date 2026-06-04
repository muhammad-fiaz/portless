@echo off
REM portless installer for Windows (cmd wrapper).
REM
REM Usage:
REM     curl -fsSL -o install.bat https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.bat
REM     install.bat
REM
REM Or for a specific version:
REM     install.bat v0.1.0
REM
REM This is a thin wrapper around install.ps1. It exists so users who
REM live in cmd can install portless without PowerShell syntax.
REM
REM Behavior:
REM   1. If install.ps1 is next to this .bat file, run it directly.
REM   2. Otherwise, download install.ps1 from the GitHub raw URL to
REM      %TEMP% and run it from there.
REM   3. Pass through any extra arguments to install.ps1.

setlocal EnableDelayedExpansion

if "%~1"=="-h" goto :help
if "%~1"=="--help" goto :help
if "%~1"=="/?" goto :help

set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

set "LOCAL_PS=%SCRIPT_DIR%\install.ps1"
set "PS_SCRIPT="

if exist "%LOCAL_PS%" (
    set "PS_SCRIPT=%LOCAL_PS%"
) else (
    set "PS_URL=https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.ps1"
    set "PS_SCRIPT=%TEMP%\portless-install-%RANDOM%.ps1"
    echo Downloading installer script ...
    curl -fsSL -o "!PS_SCRIPT!" "!PS_URL!"
    if errorlevel 1 (
        echo ERROR: could not download install.ps1 1>&2
        exit /b 1
    )
)

set "PS_EXE="
where pwsh >nul 2>nul
if %ERRORLEVEL% == 0 set "PS_EXE=pwsh"
where powershell >nul 2>nul
if %ERRORLEVEL% == 0 if "!PS_EXE!" == "" set "PS_EXE=powershell"

if "!PS_EXE!" == "" (
    echo ERROR: PowerShell is not installed or not on PATH. 1>&2
    echo Install PowerShell from https://aka.ms/install-powershell 1>&2
    exit /b 1
)

"!PS_EXE!" -NoProfile -ExecutionPolicy Bypass -File "!PS_SCRIPT!" %*
endlocal
exit /b %ERRORLEVEL%

:help
echo Usage: install.bat [version]
echo.
echo Examples:
echo     install.bat           Install the latest release
echo     install.bat v0.1.0    Install a specific release tag
exit /b 0
