pub use libcap_sys::{
    cap_flag_t_CAP_INHERITABLE as CAP_INHERITABLE, cap_flag_t_CAP_PERMITTED as CAP_PERMITTED,
    cap_flag_value_t_CAP_SET as CAP_SET,
};

pub struct Capabilities {
    cap_p: libcap_sys::cap_t,
}

impl Capabilities {
    pub fn get_proc() -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let caps = libcap_sys::cap_get_proc();
            if caps.is_null() {
                return Err(Box::new(std::io::Error::last_os_error()));
            }

            Ok(Capabilities { cap_p: caps })
        }
    }

    pub fn set_proc(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            if libcap_sys::cap_set_proc(self.cap_p) == -1 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }

        Ok(())
    }

    pub fn get_flag(
        &self,
        cap: libcap_sys::cap_value_t,
        flag: libcap_sys::cap_flag_t,
    ) -> Result<libcap_sys::cap_flag_value_t, Box<dyn std::error::Error>> {
        let mut value = 0;

        unsafe {
            if libcap_sys::cap_get_flag(self.cap_p, cap, flag, &mut value) == -1 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }

        Ok(value)
    }

    pub fn set_flag(
        &mut self,
        flag: libcap_sys::cap_flag_t,
        caps: &[libcap_sys::cap_value_t],
        value: libcap_sys::cap_flag_value_t,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let caps_len: std::os::raw::c_int = match caps.len().try_into() {
            Ok(len) => len,
            Err(e) => return Err(Box::new(e)),
        };

        unsafe {
            if libcap_sys::cap_set_flag(self.cap_p, flag, caps_len, caps.as_ptr(), value) == -1 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }

        Ok(())
    }
}

impl std::ops::Drop for Capabilities {
    fn drop(&mut self) {
        unsafe {
            libcap_sys::cap_free(self.cap_p as *mut std::os::raw::c_void);
        }
    }
}

pub fn get_bound(cap: libcap_sys::cap_value_t) -> Option<bool> {
    unsafe {
        match libcap_sys::cap_get_bound(cap) {
            -1 => None,
            0 => Some(false),
            _ => Some(true),
        }
    }
}

pub fn set_ambient(
    cap: libcap_sys::cap_value_t,
    value: libcap_sys::cap_flag_value_t,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if libcap_sys::cap_set_ambient(cap, value) == -1 {
            return Err(Box::new(std::io::Error::last_os_error()));
        }
    }

    Ok(())
}

pub fn max_bits() -> libcap_sys::cap_value_t {
    unsafe { libcap_sys::cap_max_bits() }
}
