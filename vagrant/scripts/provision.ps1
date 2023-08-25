iex ((new-object net.webclient).DownloadString('https://chocolatey.org/install.ps1'))
choco install -y --force googlechrome
choco install -y --force python
choco install -y --force nushell

(new-object net.webclient).DownloadFile("https://downloads.robocorp.com/rcc/releases/v14.6.0/windows64/rcc.exe", "C:\Users\vagrant\AppData\Local\Microsoft\WindowsApps\rcc.exe")

python -m pip install -e "C:\robotmk\"
