use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::{Builder, Uuid, uuid};

/// The unique id of the machine the scheduler runs on, if it can be determined.
///
/// The id of the operating system (machine-id(5) on Linux, the MachineGuid registry value on
/// Windows) is confidential and must not leave the machine. The reported id is therefore derived
/// from it with a keyed hash and a fixed, application-specific key, the way
/// sd_id128_get_machine_app_specific(3) does it.
///
/// The id is only as unique as the id of the operating system. Clones of a virtual machine that
/// keep its machine-id or MachineGuid report the same id. Regenerating the id in the template
/// before cloning is the operator's job, the scheduler does not work around it.
pub fn machine_id() -> Option<String> {
    read().map(app_specific_id)
}

const APP_ID: Uuid = uuid!("bc3b85b5-fb53-4d94-ab1e-05fa29ec86b9");

fn app_specific_id(machine_id: Uuid) -> String {
    let mut hmac = Hmac::<Sha256>::new_from_slice(machine_id.as_bytes())
        .expect("HMAC accepts keys of any length");
    hmac.update(APP_ID.as_bytes());
    let digest = hmac.finalize().into_bytes();
    Builder::from_random_bytes(digest[..16].try_into().expect("digest has 32 bytes"))
        .into_uuid()
        .as_simple()
        .to_string()
}

#[cfg(unix)]
fn read() -> Option<Uuid> {
    ["/etc/machine-id", "/var/lib/dbus/machine-id"]
        .into_iter()
        .find_map(|path| os_id(&std::fs::read_to_string(path).ok()?))
}

#[cfg(windows)]
fn read() -> Option<Uuid> {
    os_id(
        &windows_registry::LOCAL_MACHINE
            .open("SOFTWARE\\Microsoft\\Cryptography")
            .ok()?
            .get_string("MachineGuid")
            .ok()?,
    )
}

fn os_id(content: &str) -> Option<Uuid> {
    Uuid::parse_str(content.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: Uuid = uuid!("9ddb78eb-fb94-4f8a-8711-dfd9f93220fa");

    #[test]
    fn app_specific_id_matches_sd_id128_get_machine_app_specific() {
        assert_eq!(app_specific_id(ID), "426f61d315ae408796589657a55d53a1");
    }

    #[test]
    fn os_id_accepts_the_content_of_machine_id() {
        assert_eq!(os_id("9ddb78ebfb944f8a8711dfd9f93220fa\n"), Some(ID));
    }

    #[test]
    fn os_id_accepts_a_machine_guid() {
        assert_eq!(os_id("9ddb78eb-fb94-4f8a-8711-dfd9f93220fa"), Some(ID));
    }

    #[test]
    fn os_id_rejects_an_uninitialized_machine_id() {
        assert_eq!(os_id("uninitialized\n"), None);
    }

    #[test]
    fn os_id_rejects_an_empty_value() {
        assert_eq!(os_id(""), None);
    }
}
