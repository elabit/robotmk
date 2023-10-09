# Dependencies for creating Virtual machine as a Linux host

- VirtualBox
- Vagrant

For provisioning the machine only:

- Ansible
- Checkmk Ansible collection found here https://galaxy.ansible.com/checkmk/general
  ```
  ansible-galaxy collection install checkmk.general
  ```

For running f12.sh only:

- Mingw-w64 of cross-compilation
  ```
  sudo apt install mingw-w64
  ```
- Rust cross-compilation target `x86_64-pc-windows-gnu`
  ```
  rustup target add x86_64-pc-windows-gnu
  ```
- sshpass
  ```
  sudo apt install sshpass
  ```
