@echo off
cd /d "%~dp0"
chcp 65001 >nul
title PoiskContact - Contact search and outreach
echo Starting PoiskContact...
if exist "poisk_contact.exe" (
    start "" "poisk_contact.exe"
) else if exist "target\release\poisk_contact.exe" (
    start "" "target\release\poisk_contact.exe"
) else (
    cargo run --locked --release --bin poisk_contact
)
