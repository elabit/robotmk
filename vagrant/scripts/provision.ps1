iex ((new-object net.webclient).DownloadString('https://chocolatey.org/install.ps1'))
choco install -y googlechrome
choco install -y python

(new-object net.webclient).DownloadFile("https://downloads.robocorp.com/rcc/releases/latest/windows64/rcc.exe", "C:\Users\vagrant\AppData\Local\Microsoft\WindowsApps\rcc.exe")
