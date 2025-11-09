#!/usr/bin/env pwsh
# Build script for nocodo Android app
# Usage: .\build.ps1 [assembleDebug|assembleRelease|clean]

param(
    [string]$Task = "assembleDebug"
)

# Configuration
$AndroidStudioPath = "C:\Program Files\Android\Android Studio"
$JavaHome = "$AndroidStudioPath\jbr"
$GradleVersion = "8.2"

# Color output functions
function Write-Success { Write-Host $args -ForegroundColor Green }
function Write-Error { Write-Host $args -ForegroundColor Red }
function Write-Info { Write-Host $args -ForegroundColor Cyan }

# Check if Android Studio JBR exists
if (-not (Test-Path $JavaHome)) {
    Write-Error "ERROR: Android Studio JBR not found at: $JavaHome"
    Write-Info "Please install Android Studio or update the `$AndroidStudioPath variable in this script"
    exit 1
}

Write-Info "Using Java from: $JavaHome"
$env:JAVA_HOME = $JavaHome

# Find Gradle wrapper distribution
$GradleHome = "$env:USERPROFILE\.gradle\wrapper\dists\gradle-$GradleVersion-bin"
if (Test-Path $GradleHome) {
    $GradleDistDir = Get-ChildItem -Path $GradleHome -Directory | Select-Object -First 1
    if ($GradleDistDir) {
        $GradleBin = "$($GradleDistDir.FullName)\gradle-$GradleVersion\bin\gradle.bat"

        if (Test-Path $GradleBin) {
            Write-Info "Using Gradle: $GradleBin"
            Write-Info "Running task: $Task"
            Write-Host ""

            # Run Gradle build
            & $GradleBin $Task

            if ($LASTEXITCODE -eq 0) {
                Write-Host ""
                Write-Success "BUILD SUCCESSFUL!"

                if ($Task -eq "assembleDebug") {
                    Write-Info "APK location: app\build\outputs\apk\debug\app-debug.apk"
                } elseif ($Task -eq "assembleRelease") {
                    Write-Info "APK location: app\build\outputs\apk\release\app-release.apk"
                }
            } else {
                Write-Host ""
                Write-Error "BUILD FAILED!"
                exit $LASTEXITCODE
            }

            exit 0
        }
    }
}

# If Gradle wrapper not found, try to use gradlew
Write-Info "Gradle $GradleVersion distribution not found in user cache"
Write-Info "Attempting to use local gradlew wrapper..."

if (Test-Path ".\gradlew") {
    # Unix-style gradlew for Git Bash/WSL
    Write-Info "Running: .\gradlew $Task"
    Write-Host ""

    bash -c "JAVA_HOME='$($env:JAVA_HOME -replace '\\','/')' ./gradlew $Task"

    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Success "BUILD SUCCESSFUL!"

        if ($Task -eq "assembleDebug") {
            Write-Info "APK location: app\build\outputs\apk\debug\app-debug.apk"
        } elseif ($Task -eq "assembleRelease") {
            Write-Info "APK location: app\build\outputs\apk\release\app-release.apk"
        }
    } else {
        Write-Host ""
        Write-Error "BUILD FAILED!"
        exit $LASTEXITCODE
    }
} else {
    Write-Error "ERROR: Neither Gradle distribution nor gradlew wrapper found"
    Write-Info "Please run this from the android-app directory"
    exit 1
}
