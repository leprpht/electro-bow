@echo off
setlocal

rem Configure ElectroBow from any checkout location.
set "PROJECT_DIR=%~dp0"
set "BUILD_DIR=%PROJECT_DIR%build"

set "CMAKE_EXE="
for /f "delims=" %%I in ('where cmake 2^>nul') do if not defined CMAKE_EXE set "CMAKE_EXE=%%I"
if not defined CMAKE_EXE if exist "C:\Program Files\CMake\bin\cmake.exe" set "CMAKE_EXE=C:\Program Files\CMake\bin\cmake.exe"
if not defined CMAKE_EXE if exist "C:\CMake\bin\cmake.exe" set "CMAKE_EXE=C:\CMake\bin\cmake.exe"
if not defined CMAKE_EXE (
    echo.
    echo CMake could not be found.
    echo.
    echo Please install CMake from https://cmake.org/download/ and choose
    echo ^"Add CMake to the system PATH^" during installation.
    echo Then close and reopen Command Prompt and run this file again.
    exit /b 1
)

if not defined JUCE_PATH if exist "%PROJECT_DIR%JUCE\CMakeLists.txt" set "JUCE_PATH=%PROJECT_DIR%JUCE"
if not defined JUCE_PATH (
    echo JUCE was not found.
    echo Download JUCE, extract it, and either place its folder here as ^"JUCE^"
    echo or set its location before running this file:
    echo   set JUCE_PATH=C:\path\to\JUCE
    exit /b 1
)

if not exist "%JUCE_PATH%\CMakeLists.txt" (
    echo The JUCE folder was not found at "%JUCE_PATH%".
    echo Check the folder location and try again.
    exit /b 1
)

"%CMAKE_EXE%" -S "%PROJECT_DIR%" -B "%BUILD_DIR%" -DJUCE_PATH="%JUCE_PATH%"
if errorlevel 1 (
    echo.
    echo Configuration failed. Check the CMake message above for details.
    echo Common causes are an invalid JUCE path or missing C++ build tools.
    exit /b 1
)

echo Configuration successful.
