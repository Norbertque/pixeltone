//! The Anthropic API key for AI playlists, kept in Windows Credential Manager (encrypted for
//! the signed-in user), never in the settings file. The UI can store or remove it, and ask
//! whether one is stored; it never gets the key back.

const TARGET: &str = "PixelPlayer PC/anthropic-api-key";

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::ERROR_NOT_FOUND;
    use windows::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    };
    use windows::core::{HSTRING, PWSTR};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    pub fn store(target: &str, secret: &str) -> Result<(), String> {
        let mut name = wide(target);
        let mut user = wide("PixelPlayer PC");
        let mut blob = secret.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(name.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        unsafe { CredWriteW(&credential, 0) }.map_err(|e| e.message())
    }

    pub fn load(target: &str) -> Result<Option<String>, String> {
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        match unsafe { CredReadW(&HSTRING::from(target), CRED_TYPE_GENERIC, None, &mut found) } {
            Ok(()) => {}
            Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => return Ok(None),
            Err(e) => return Err(e.message()),
        }
        let secret = unsafe {
            let c = &*found;
            let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
            let s = String::from_utf8_lossy(bytes).into_owned();
            CredFree(found as *const _);
            s
        };
        Ok(Some(secret))
    }

    pub fn delete(target: &str) -> Result<(), String> {
        match unsafe { CredDeleteW(&HSTRING::from(target), CRED_TYPE_GENERIC, None) } {
            Err(e) if e.code() != ERROR_NOT_FOUND.to_hresult() => Err(e.message()),
            _ => Ok(()),
        }
    }
}

/// Off Windows (development builds only) the key lives in memory until the app quits.
#[cfg(not(windows))]
mod imp {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn vault() -> &'static Mutex<HashMap<String, String>> {
        static VAULT: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        VAULT.get_or_init(Default::default)
    }

    pub fn store(target: &str, secret: &str) -> Result<(), String> {
        vault()
            .lock()
            .map_err(|e| e.to_string())?
            .insert(target.into(), secret.into());
        Ok(())
    }

    pub fn load(target: &str) -> Result<Option<String>, String> {
        Ok(vault()
            .lock()
            .map_err(|e| e.to_string())?
            .get(target)
            .cloned())
    }

    pub fn delete(target: &str) -> Result<(), String> {
        vault().lock().map_err(|e| e.to_string())?.remove(target);
        Ok(())
    }
}

pub fn store_api_key(key: &str) -> Result<(), String> {
    imp::store(TARGET, key.trim())
}

pub fn api_key() -> Result<Option<String>, String> {
    Ok(imp::load(TARGET)?.filter(|k| !k.trim().is_empty()))
}

pub fn delete_api_key() -> Result<(), String> {
    imp::delete(TARGET)
}
