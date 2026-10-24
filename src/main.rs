mod capabilities;

use std::{ffi::CString, io::Write, os::unix::ffi::OsStrExt};

use capabilities::{CAP_INHERITABLE, CAP_PERMITTED, CAP_SET, Capabilities};

fn back_to_original_user() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // securebitsを取得
        let mut securebits = libc::prctl(libc::PR_GET_SECUREBITS);
        if securebits == -1 {
            return Err(Box::new(std::io::Error::last_os_error()));
        }

        // securebits操作後に現在の状態に戻せるように現在の状態を取っておく
        let original_no_setuid_fixup = securebits & libc::SECBIT_NO_SETUID_FIXUP != 0;

        // 非rootにsetuidしてもcapabilityを保持できるようにする
        if !original_no_setuid_fixup {
            securebits |= libc::SECBIT_NO_SETUID_FIXUP;
            if libc::prctl(libc::PR_SET_SECUREBITS, securebits) == -1 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }

        let mut error = None;
        'edited_securebits: {
            // 元のユーザーに戻す
            let gid = libc::getgid();
            if libc::setgid(gid) == -1 {
                error = Some(Box::new(std::io::Error::last_os_error()));
                break 'edited_securebits;
            }

            let uid = libc::getuid();
            if libc::setuid(uid) == -1 {
                error = Some(Box::new(std::io::Error::last_os_error()));
                break 'edited_securebits;
            }
        }

        // 非rootにsetuidするとcapabilityを失うデフォルトの状態に戻す
        if !original_no_setuid_fixup {
            securebits &= !libc::SECBIT_NO_SETUID_FIXUP;
            if libc::prctl(libc::PR_SET_SECUREBITS, securebits) == -1 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }

        if let Some(e) = error {
            return Err(e);
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = back_to_original_user() {
        eprintln!("Failed to back to original user: {}", e);
        std::process::exit(1);
    }

    // 現在のcapabilityを取得する
    let mut caps = match Capabilities::get_proc() {
        Ok(caps) => caps,
        Err(e) => {
            eprintln!("Failed to get current capabilities: {}", e);
            std::process::exit(1);
        }
    };

    // inheritableに追加可能な全てのcapabilityを追加する
    for cap in 0..capabilities::max_bits() {
        let bound = match capabilities::get_bound(cap) {
            Some(b) => b,
            None => {
                eprintln!("Failed to get bound for capability {}", cap);
                std::process::exit(1);
            }
        };

        if bound && let Err(e) = caps.set_flag(CAP_INHERITABLE, &[cap], CAP_SET) {
            eprintln!(
                "Failed to set inheritable flag for capability {}: {}",
                cap, e
            );
            std::process::exit(1);
        }
    }
    if let Err(e) = caps.set_proc() {
        eprintln!("Failed to set inheritable capabilities: {}", e);
        std::process::exit(1);
    }

    // permittedとinheritableの両方がCAP_SETのcapabilityをambientに追加する
    for cap in 0..capabilities::max_bits() {
        let permitted = match caps.get_flag(cap, CAP_PERMITTED) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("Failed to get permitted flag for capability {}: {}", cap, e);
                std::process::exit(1);
            }
        };

        let inheritable = match caps.get_flag(cap, CAP_INHERITABLE) {
            Ok(value) => value,
            Err(e) => {
                eprintln!(
                    "Failed to get inheritable flag for capability {}: {}",
                    cap, e
                );
                std::process::exit(1);
            }
        };

        // permittedとinheritableの両方がCAP_SETの場合にのみ, ambientに追加する
        if permitted == CAP_SET
            && inheritable == CAP_SET
            && let Err(e) = capabilities::set_ambient(cap, CAP_SET)
        {
            eprintln!("Failed to set ambient flag for capability {}: {}", cap, e);
            std::process::exit(1);
        }
    }

    let args: Vec<_> = std::env::args_os()
        .map(|v| CString::new(v.as_bytes()).unwrap())
        .collect();
    if args.len() < 2 {
        let mut stderr = std::io::stderr();
        stderr.write_all(b"Usage: ").unwrap();
        stderr
            .write_all(args.first().map_or(b"uac", |v| v.as_bytes()))
            .unwrap();
        stderr.write_all(b" <command> [args...]\n").unwrap();
        return;
    }

    let argv: Vec<_> = args
        .iter()
        .skip(1)
        .map(|v| v.as_ptr())
        .chain([std::ptr::null()])
        .collect();
    unsafe {
        if libc::execvp(argv[0], argv.as_ptr()) == -1 {
            eprintln!(
                "Failed to execute the command: {}",
                std::io::Error::last_os_error()
            );
            std::process::exit(1);
        }
    }
}
