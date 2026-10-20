@echo off
setlocal EnableDelayedExpansion
chcp 65001 >nul
cd /d "%~dp0"

set "DESTINO=%LOCALAPPDATA%\Programs\8Pace"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo === 8Pace: build ===

rem --- ambiente do compilador C++ (MSVC) ---
rem Procura uma instalacao do Visual Studio que tenha as libs x64 completas.
where cl >nul 2>&1
if errorlevel 1 (
    set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
    set "VCVARS="
    for /f "usebackq delims=" %%i in (`"!VSWHERE!" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do (
        if not defined VCVARS (
            dir /s /b "%%i\VC\Tools\MSVC\msvcrt.lib" 2>nul | findstr /i "\\lib\\x64\\msvcrt.lib" >nul && set "VCVARS=%%i\VC\Auxiliary\Build\vcvars64.bat"
        )
    )
    if not defined VCVARS (
        echo [ERRO] Nenhum Visual Studio com "Desenvolvimento para desktop com C++" completo foi encontrado.
        goto :falha
    )
    call "!VCVARS!" >nul || goto :falha
)

rem --- dependencias do frontend ---
if not exist node_modules (
    echo Instalando dependencias npm...
    call npm install || goto :falha
)

rem --- build do executavel (sem instalador) ---
call npm run tauri build -- --no-bundle || goto :falha

set "EXE=src-tauri\target\release\8Pace.exe"
if not exist "%EXE%" (
    echo [ERRO] Executavel nao encontrado em %EXE%
    goto :falha
)

rem --- instala em %LOCALAPPDATA%\Programs\8Pace ---
echo Instalando em %DESTINO%...
taskkill /IM 8Pace.exe /F >nul 2>&1
ping -n 2 127.0.0.1 >nul
if not exist "%DESTINO%" mkdir "%DESTINO%"
copy /y "%EXE%" "%DESTINO%\8Pace.exe" >nul || goto :falha

rem --- atalhos: area de trabalho e menu Iniciar ---
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$exe = Join-Path $env:LOCALAPPDATA 'Programs\8Pace\8Pace.exe';" ^
  "$sh = New-Object -ComObject WScript.Shell;" ^
  "$destinos = @([Environment]::GetFolderPath('Desktop'), (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'));" ^
  "foreach ($d in $destinos) { New-Item -ItemType Directory -Force $d | Out-Null; $l = $sh.CreateShortcut((Join-Path $d '8Pace.lnk')); $l.TargetPath = $exe; $l.WorkingDirectory = Split-Path $exe; $l.IconLocation = \"$exe,0\"; $l.Description = '8Pace - tracking de atividades'; $l.Save() }" ^
  || goto :falha

echo.
echo Pronto. 8Pace instalado em %DESTINO%
echo Atalhos criados na area de trabalho e no menu Iniciar.
echo.
echo Barra de tarefas: o Windows 11 nao permite fixar por script.
echo Na primeira vez, abra o 8Pace, clique com o botao direito no icone
echo da barra de tarefas e escolha "Fixar na barra de tarefas".
echo Como o caminho do executavel nao muda, o atalho fixado continua
echo valendo nos proximos builds.
echo.
start "" "%DESTINO%\8Pace.exe"
pause
exit /b 0

:falha
echo.
echo [ERRO] O build falhou.
pause
exit /b 1
