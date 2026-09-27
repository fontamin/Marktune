@echo off
setlocal

:: Usage: python marktune_fea_generator.py <NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]

echo Usage: python marktune_fea_generator.py ^<NUM_ROWS^> ^<STEP_PERCENT^> ^<UPM^> [output.fea]

set "SCRIPT_DIR=%~dp0"
set "PY_SCRIPT=%SCRIPT_DIR%marktune_fea_generator.py"

cd /d "%SCRIPT_DIR%"

python "%PY_SCRIPT%" 24 4 1000

echo Press enter to continue
pause >nul

endlocal
