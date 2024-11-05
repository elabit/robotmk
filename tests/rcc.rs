use camino::Utf8Path;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Deserialize)]
pub struct ConfigurationDiagnostics {
    pub details: HashMap<String, String>,
}

pub fn read_configuration_diagnostics(
    binary_path: &Utf8Path,
    robocorp_home: &str,
) -> anyhow::Result<ConfigurationDiagnostics> {
    let mut config_diag_command = Command::new(binary_path);
    config_diag_command
        .arg("configuration")
        .arg("diagnostics")
        .arg("--json")
        .env("ROBOCORP_HOME", robocorp_home);
    let stdout = String::from_utf8(config_diag_command.output()?.stdout)?;
    let diagnostics: ConfigurationDiagnostics = serde_json::from_str(&stdout)?;
    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIGURATION_DIAGNOSTICS_JSON: &str = r#"{
      "details": {
        "ENV:ComSpec": "",
        "ENV:LANG": "en_US.UTF-8",
        "ENV:SHELL": "/usr/bin/zsh",
        "RCC_VERBOSE_ENVIRONMENT_BUILDING": "false",
        "ROBOCORP_HOME": "/home/solo/.robocorp",
        "ROBOCORP_OVERRIDE_SYSTEM_REQUIREMENTS": "false",
        "config-active-profile": "default",
        "config-http-proxy": "",
        "config-https-proxy": "",
        "config-micromambarc-used": "false",
        "config-piprc-used": "false",
        "config-settings-yaml-used": "false",
        "config-ssl-no-revoke": "false",
        "config-ssl-verify": "true",
        "controller": "rcc.user",
        "cpus": "8",
        "dns-lookup-time": "DNS lookup time for 9 hostnames was about 0.071s",
        "executable": "/usr/local/bin/rcc",
        "hololib-catalog-location": "/home/solo/.robocorp/hololib/catalog",
        "hololib-library-location": "/home/solo/.robocorp/hololib/library",
        "hololib-location": "/home/solo/.robocorp/hololib",
        "holotree-global-shared": "false",
        "holotree-location": "/home/solo/.robocorp/holotree",
        "holotree-shared": "false",
        "holotree-user-id": "82055ed",
        "installationId": "3aaeb548-62aa-1b15-5095-f56474962403",
        "lock-cache": "/home/solo/.robocorp/rcccache.yaml.lck",
        "lock-config": "/home/solo/.robocorp/rcc.yaml.lck",
        "lock-holotree": "/home/solo/.robocorp/holotree/global.lck",
        "lock-robocorp": "/home/solo/.robocorp/robocorp.lck",
        "lock-userlock": "/home/solo/.robocorp/holotree/82055ed_5a1fac3_9fcd2534.lck",
        "micromamba": "1.4.2",
        "micromamba.bin": "/home/solo/.robocorp/bin/micromamba",
        "no-build": "false",
        "os": "linux_amd64",
        "os-holo-location": "/opt/robocorp/ht",
        "rcc": "v14.15.4",
        "rcc.bin": "/usr/local/bin/rcc",
        "telemetry-enabled": "false",
        "tempdir": "/tmp",
        "uid:gid": "1000:1000",
        "user-agent": "rcc/v14.15.4 (linux amd64) rcc.user",
        "user-cache-dir": "/home/solo/.cache",
        "user-config-dir": "/home/solo/.config",
        "user-home-dir": "/home/solo",
        "when": "2024-06-13T15:49:34+02:00 (CEST)",
        "working-dir": "/home/solo/git/rcc"
      },
      "checks": [
        {
          "type": "RPA",
          "category": 3010,
          "status": "ok",
          "message": "ROBOCORP_HOME (/home/solo/.robocorp) is good enough.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1030,
          "status": "ok",
          "message": "PYTHONPATH is not set, which is good.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1030,
          "status": "ok",
          "message": "PLAYWRIGHT_BROWSERS_PATH is not set, which is good.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1030,
          "status": "ok",
          "message": "NODE_OPTIONS is not set, which is good.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1030,
          "status": "ok",
          "message": "NODE_PATH is not set, which is good.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1010,
          "status": "ok",
          "message": "Supports long enough paths.",
          "url": "https://robocorp.com/docs/troubleshooting/windows-long-path"
        },
        {
          "type": "OS",
          "category": 1021,
          "status": "ok",
          "message": "Possibly pending lock \"rcccache.yaml.lck\", user: \"solo\", space: \"user\", and controller: \"user\" (parent/pid: 942155/983022). May cause environment wait/build delay.",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "OS",
          "category": 1020,
          "status": "ok",
          "message": "5 lockfiles all seem to work correctly (for this user).",
          "url": "https://robocorp.com/docs/troubleshooting"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "api.eu1.robocorp.com found [DNS query]: [54.229.42.247 63.32.3.75 54.194.229.229]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "cloud.robocorp.com found [DNS query]: [52.208.5.184 52.214.242.60 34.254.101.112]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "conda.anaconda.org found [DNS query]: [104.19.144.37 104.19.145.37 2606:4700::6813:9125 2606:4700::6813:9025]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "downloads.robocorp.com found [DNS query]: [172.67.7.153 104.22.41.65 104.22.40.65 2606:4700:10::ac43:799 2606:4700:10::6816:2841 2606:4700:10::6816:2941]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "files.pythonhosted.org found [DNS query]: [146.75.120.223 2a04:4e42:8e::223]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "github.com found [DNS query]: [140.82.121.4]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "pypi.org found [DNS query]: [151.101.0.223 151.101.128.223 151.101.192.223 151.101.64.223 2a04:4e42:600::223 2a04:4e42:400::223 2a04:4e42::223 2a04:4e42:200::223]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "robocorp.com found [DNS query]: [104.22.41.65 104.22.40.65 172.67.7.153 2606:4700:10::ac43:799 2606:4700:10::6816:2841 2606:4700:10::6816:2941]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4010,
          "status": "ok",
          "message": "telemetry.robocorp.com found [DNS query]: [54.217.65.213 52.50.58.133]",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4040,
          "status": "ok",
          "message": "Canary download successful [GET request]: https://downloads.robocorp.com/canary.txt",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4030,
          "status": "ok",
          "message": "PyPI canary download successful [HEAD request]: https://pypi.org/jupyterlab-pygments/",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "network",
          "category": 4030,
          "status": "ok",
          "message": "Conda canary download successful [HEAD request]: https://conda.anaconda.org/conda-forge/linux-64/repodata.json",
          "url": "https://robocorp.com/docs/troubleshooting/firewall-and-proxies"
        },
        {
          "type": "Settings",
          "category": 0,
          "status": "ok",
          "message": "In general, 'settings.yaml' is ok.",
          "url": ""
        }
      ]
    }"#;

    #[test]
    fn serialize() {
        let diagnostic: ConfigurationDiagnostics =
            serde_json::from_str(CONFIGURATION_DIAGNOSTICS_JSON).unwrap();
        assert_eq!(
            diagnostic
                .details
                .get("holotree-location")
                .unwrap()
                .as_str(),
            "/home/solo/.robocorp/holotree"
        )
    }
}
