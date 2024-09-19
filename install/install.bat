@echo off
setlocal

rem Set your repository and version
set REPO=bracketengineering/bracket-cli
set VERSION=latest

rem Detect architecture
for /f "tokens=*" %%i in ('wmic os get osarchitecture') do set ARCH=%%i

rem Determine the appropriate binary to download
if %ARCH%==64-bit (
    set FILE=bracket-cli-windows-amd64.exe
) else (
    echo Unsupported architecture: %ARCH%
    exit /b 1
)

rem Download the binary
set URL=https://github.com/%REPO%/releases/download/%VERSION%/%FILE%
powershell -Command "Invoke-WebRequest -Uri %URL% -OutFile %TEMP%\bracket.exe"

rem Move the binary to a directory in PATH
move /Y %TEMP%\bracket.exe C:\Windows\System32\bracket.exe

rem Verify installation
where bracket >nul 2>&1
if %errorlevel%==0 (
    echo Installation successful!
) else (
    echo Installation failed.
    exit /b 1
)

endlocal
