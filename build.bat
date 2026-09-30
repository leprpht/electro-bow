@echo off
setlocal

rem Build ElectroBow from any checkout location.
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

if not exist "%BUILD_DIR%\CMakeCache.txt" (
    call "%PROJECT_DIR%configure.bat"
    if errorlevel 1 exit /b 1
)

"%CMAKE_EXE%" --build "%BUILD_DIR%" --config Release --parallel
if errorlevel 1 (
    echo.
    echo Build failed. Check the compiler message above for details.
    echo Make sure Visual Studio has the Desktop development with C++ workload installed.
    exit /b 1
)

echo Build successful.
