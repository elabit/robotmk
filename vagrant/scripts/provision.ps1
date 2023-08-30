iex ((new-object net.webclient).DownloadString('https://chocolatey.org/install.ps1'))
choco install -y --force googlechrome
choco install -y --force python
choco install -y --force nushell

(new-object net.webclient).DownloadFile("https://nightly.link/elabit/robotmk/actions/artifacts/878920132.zip", "C:\Users\vagrant\Downloads\rcc.zip")
Expand-Archive "C:\Users\vagrant\Downloads\rcc.zip" -Force -DestinationPath "C:\Users\vagrant\Downloads\"
Copy-Item "C:\Users\vagrant\Downloads\windows64\rcc.exe" -Destination "C:\Users\vagrant\AppData\Local\Microsoft\WindowsApps\rcc.exe"

python -m pip install -e "C:\robotmk\"
