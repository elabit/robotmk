iex ((new-object net.webclient).DownloadString('https://chocolatey.org/install.ps1'))

choco install -y python --version 3.10.6
choco install -y vscode git.install 7zip sysinternals firefox googlechrome sandboxie synctrayzor meld

$o = new-object -com shell.application
$o.Namespace('C:\ProgramData\chocolatey\lib\sysinternals\tools').Self.InvokeVerb("SysInternals")

Set-WinUILanguageOverride -Language de-DE
Set-SystemPreferredUILanguage de-DE

# # plugin from github.com/frankus0512/vagrant-esxi branch newfeatures
# $env:Path = $env:Path + ";C:\HashiCorp\Vagrant\bin"
# vagrant plugin install c:\vagrant\plugins\vagrant-esxi-0.1.1.gem

# # Download vagrant SSH key to login to ESXi server
# If (! (Test-Path "C:\Users\vagrant\.ssh")) {
#   New-Item -Path "C:\Users\vagrant\.ssh" -ItemType Directory
# }
# (New-Object System.Net.WebClient).DownloadFile('https://raw.githubusercontent.com/mitchellh/vagrant/master/keys/vagrant.pub', 'C:\Users\vagrant\.ssh\id_rsa.pub')
# (New-Object System.Net.WebClient).DownloadFile('https://raw.githubusercontent.com/mitchellh/vagrant/master/keys/vagrant', 'C:\Users\vagrant\.ssh\id_rsa')
