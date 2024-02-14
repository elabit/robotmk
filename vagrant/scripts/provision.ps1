iex ((new-object net.webclient).DownloadString('https://chocolatey.org/install.ps1'))
choco install -y --force googlechrome
choco install -y --force python
