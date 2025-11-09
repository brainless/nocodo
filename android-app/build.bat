@echo off
REM Build script for nocodo Android app
REM Usage: build.bat [assembleDebug|assembleRelease|clean]

setlocal

REM Set default task
set TASK=%1
if "%TASK%"=="" set TASK=assembleDebug

REM Configuration
set ANDROID_STUDIO_PATH=C:\Program Files\Android\Android Studio
set JAVA_HOME=%ANDROID_STUDIO_PATH%\jbr
set GRADLE_VERSION=8.2

REM Check if Android Studio JBR exists
if not exist "%JAVA_HOME%" (
    echo ERROR: Android Studio JBR not found at: %JAVA_HOME%
    echo Please install Android Studio or update ANDROID_STUDIO_PATH in this script
    exit /b 1
)

echo Using Java from: %JAVA_HOME%
echo.

REM Find Gradle wrapper distribution
set GRADLE_HOME=%USERPROFILE%\.gradle\wrapper\dists\gradle-%GRADLE_VERSION%-bin

if exist "%GRADLE_HOME%" (
    for /d %%i in ("%GRADLE_HOME%\*") do (
        set GRADLE_BIN=%%i\gradle-%GRADLE_VERSION%\bin\gradle.bat
        goto :found_gradle
    )
)

:found_gradle
if exist "%GRADLE_BIN%" (
    echo Using Gradle: %GRADLE_BIN%
    echo Running task: %TASK%
    echo.

    call "%GRADLE_BIN%" %TASK%

    if errorlevel 1 (
        echo.
        echo BUILD FAILED!
        exit /b 1
    ) else (
        echo.
        echo BUILD SUCCESSFUL!
        if "%TASK%"=="assembleDebug" echo APK location: app\build\outputs\apk\debug\app-debug.apk
        if "%TASK%"=="assembleRelease" echo APK location: app\build\outputs\apk\release\app-release.apk
    )
    exit /b 0
)

REM If Gradle not found, try gradlew
echo Gradle %GRADLE_VERSION% distribution not found in user cache
echo Attempting to use local gradlew wrapper...
echo.

if exist "gradlew" (
    bash -c "JAVA_HOME='%JAVA_HOME:\=/% ' ./gradlew %TASK%"

    if errorlevel 1 (
        echo.
        echo BUILD FAILED!
        exit /b 1
    ) else (
        echo.
        echo BUILD SUCCESSFUL!
        if "%TASK%"=="assembleDebug" echo APK location: app\build\outputs\apk\debug\app-debug.apk
        if "%TASK%"=="assembleRelease" echo APK location: app\build\outputs\apk\release\app-release.apk
    )
) else (
    echo ERROR: Neither Gradle distribution nor gradlew wrapper found
    echo Please run this from the android-app directory
    exit /b 1
)

endlocal
