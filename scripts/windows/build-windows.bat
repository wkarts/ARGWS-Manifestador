@echo off
setlocal EnableExtensions
cd /d "%~dp0\..\.."
set "PS_EXE=%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe"
set "PS_SCRIPT=%~dp0build-windows.ps1"
if not exist "%PS_EXE%" (
  echo Windows PowerShell nao encontrado: "%PS_EXE%"
  exit /b 1
)
if not exist "%PS_SCRIPT%" (
  echo Script de build nao encontrado: "%PS_SCRIPT%"
  exit /b 1
)
"%PS_EXE%" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%PS_SCRIPT%" %*
set EXIT_CODE=%ERRORLEVEL%
if not "%EXIT_CODE%"=="0" (
  echo.
  echo Build Windows falhou com codigo %EXIT_CODE%.
)
exit /b %EXIT_CODE%
