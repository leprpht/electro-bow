@echo off
setlocal

set "PROJECT_DIR=%~dp0"
set "UI_DIR=%PROJECT_DIR%UI"

if not defined JUCE_VERSION set "JUCE_VERSION=9.0.2"
if not defined JUCE_PATH set "JUCE_PATH=%PROJECT_DIR%JUCE"

echo Checking required commands...

for %%C in (git node npm python cmake clang-format) do (
    where %%C >nul 2>&1
    if errorlevel 1 (
        echo setup: required command '%%C' was not found in PATH.
        exit /b 1
    )
    echo setup: found %%C.
)

if exist "%JUCE_PATH%\CMakeLists.txt" goto juce_found
if exist "%JUCE_PATH%" goto invalid_juce_path

echo JUCE %JUCE_VERSION% was not found. Downloading it to "%JUCE_PATH%"...

set "JUCE_ARCHIVE=%TEMP%\JUCE-%JUCE_VERSION%.zip"
set "JUCE_EXTRACT=%TEMP%\ElectroBow-JUCE-%JUCE_VERSION%"

if exist "%JUCE_EXTRACT%" rmdir /s /q "%JUCE_EXTRACT%"

powershell -NoProfile -ExecutionPolicy Bypass -Command "Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/juce-framework/JUCE/archive/refs/tags/%JUCE_VERSION%.zip' -OutFile '%JUCE_ARCHIVE%'"
if errorlevel 1 goto setup_failed

powershell -NoProfile -ExecutionPolicy Bypass -Command "Expand-Archive -Force -Path '%JUCE_ARCHIVE%' -DestinationPath '%JUCE_EXTRACT%'"
if errorlevel 1 goto setup_failed

if not exist "%JUCE_EXTRACT%\JUCE-%JUCE_VERSION%\CMakeLists.txt" goto invalid_archive

move "%JUCE_EXTRACT%\JUCE-%JUCE_VERSION%" "%JUCE_PATH%" >nul
if errorlevel 1 goto setup_failed

del /q "%JUCE_ARCHIVE%" >nul 2>&1
goto juce_found

:invalid_juce_path
echo setup: JUCE_PATH exists but is not a JUCE checkout: "%JUCE_PATH%"
echo Remove it or set JUCE_PATH to another location, then retry.
exit /b 1

:invalid_archive
echo setup: downloaded JUCE archive has an unexpected layout.
exit /b 1

:juce_found
echo setup: found JUCE checkout at "%JUCE_PATH%".

echo Installing UI dependencies...
call npm ci --prefix "%UI_DIR%"
if errorlevel 1 exit /b 1

echo Enabling repository Git hooks (pre-commit and pre-push)...
git -C "%PROJECT_DIR%" config core.hooksPath .githooks
if errorlevel 1 exit /b 1

echo Setup complete. JUCE: %JUCE_PATH%
echo Run build.py "%JUCE_PATH%" or build.bat to configure and build the plugin.

exit /b 0

:setup_failed
echo setup: an operation failed. Check the errors above.
exit /b 1