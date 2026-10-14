//! Announces the PC on the local network as `<name>._pixeltone._tcp.local`, so phones find it
//! without typing an address. On Windows this goes through the system's own mDNS responder
//! (no extra firewall rule needed for it); elsewhere it's a no-op (development builds).

pub const SERVICE: &str = "_pixeltone._tcp";

/// Instance names can't contain dots (they separate the labels).
fn label(name: &str) -> String {
    let cleaned: String = name.trim().replace('.', "-").chars().take(60).collect();
    if cleaned.is_empty() {
        "Pixeltone".into()
    } else {
        cleaned
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::Dns::{
        DNS_QUERY_REQUEST_VERSION1, DNS_SERVICE_INSTANCE, DNS_SERVICE_REGISTER_REQUEST,
        DnsServiceConstructInstance, DnsServiceDeRegister, DnsServiceFreeInstance,
        DnsServiceRegister,
    };
    use windows::core::{HSTRING, PCWSTR};

    /// `DNS_REQUEST_PENDING`: the registration continues in the background.
    const PENDING: u32 = 9506;

    pub struct Advertisement {
        request: Box<DNS_SERVICE_REGISTER_REQUEST>,
    }

    // The request is only read by the system's DNS client; the handle moves between threads
    // only to be dropped.
    unsafe impl Send for Advertisement {}
    unsafe impl Sync for Advertisement {}

    unsafe extern "system" fn completed(
        status: u32,
        _context: *const c_void,
        instance: *const DNS_SERVICE_INSTANCE,
    ) {
        if status != 0 {
            tracing::warn!("mDNS announcement: status {status}");
        }
        if !instance.is_null() {
            // A copy for us to free (as Microsoft's sample does).
            unsafe { DnsServiceFreeInstance(instance) };
        }
    }

    impl Advertisement {
        pub fn start(
            name: &str,
            host: &str,
            port: u16,
            txt: &[(&str, &str)],
        ) -> Result<Self, String> {
            let service = HSTRING::from(format!("{}.{}.local", super::label(name), super::SERVICE));
            let host = HSTRING::from(format!("{host}.local"));
            let keys: Vec<HSTRING> = txt.iter().map(|(k, _)| HSTRING::from(*k)).collect();
            let values: Vec<HSTRING> = txt.iter().map(|(_, v)| HSTRING::from(*v)).collect();
            let key_ptrs: Vec<PCWSTR> = keys.iter().map(|k| PCWSTR(k.as_ptr())).collect();
            let value_ptrs: Vec<PCWSTR> = values.iter().map(|v| PCWSTR(v.as_ptr())).collect();
            // No addresses: Windows answers for `<host>.local` itself, with every address it has.
            let instance = unsafe {
                DnsServiceConstructInstance(
                    &service,
                    &host,
                    None,
                    None,
                    port,
                    0,
                    0,
                    key_ptrs.len() as u32,
                    key_ptrs.as_ptr(),
                    value_ptrs.as_ptr(),
                )
            };
            if instance.is_null() {
                return Err("can't describe the service for mDNS".into());
            }
            let request = Box::new(DNS_SERVICE_REGISTER_REQUEST {
                Version: DNS_QUERY_REQUEST_VERSION1.0,
                InterfaceIndex: 0,
                pServiceInstance: instance,
                pRegisterCompletionCallback: Some(completed),
                pQueryContext: std::ptr::null_mut(),
                hCredentials: HANDLE::default(),
                unicastEnabled: false.into(),
            });
            let status = unsafe { DnsServiceRegister(&*request, None) };
            if status != PENDING {
                unsafe { DnsServiceFreeInstance(instance) };
                return Err(format!("mDNS announcement failed (status {status})"));
            }
            Ok(Self { request })
        }
    }

    impl Drop for Advertisement {
        fn drop(&mut self) {
            // The instance stays allocated: the system may still read it while it withdraws
            // the announcement (a few hundred bytes, once per server stop).
            unsafe { DnsServiceDeRegister(&*self.request, None) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Advertisement;

    impl Advertisement {
        pub fn start(
            name: &str,
            host: &str,
            port: u16,
            _txt: &[(&str, &str)],
        ) -> Result<Self, String> {
            tracing::debug!(
                "not announcing {}.{}.local on {host}:{port} (Windows only)",
                super::label(name),
                super::SERVICE
            );
            Ok(Self)
        }
    }
}

pub use imp::Advertisement;

#[cfg(test)]
mod tests {
    #[test]
    fn instance_names_are_one_label() {
        assert_eq!(super::label("DESKTOP-AB12"), "DESKTOP-AB12");
        assert_eq!(super::label("my.pc"), "my-pc");
        assert_eq!(super::label("  "), "Pixeltone");
    }
}
