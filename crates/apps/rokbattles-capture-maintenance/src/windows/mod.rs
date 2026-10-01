mod machine;
mod peer;
mod scm;
#[cfg(target_arch = "x86_64")]
mod signature;

use crate::lifecycle::Maintenance;
use rokbattles_capture_ipc::windows::{InstalledFile, ProtectedInstallation};
use std::{
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle},
    },
    path::Path,
    ptr,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::*,
    System::{
        Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId},
        Threading::GetCurrentProcessId,
    },
};
const PIPE: &str = r"\\.\pipe\ROKBattles.Capture.Maintenance.v1";
const IO_TIMEOUT: Duration = Duration::from_secs(120);
const SESSION_TIMEOUT: Duration = Duration::from_secs(20 * 60);
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture maintenance trust rejected")
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn own(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: every caller transfers one newly obtained Windows handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}
fn same_path(a: &Path, b: &Path) -> bool {
    let a: Vec<_> = a.as_os_str().encode_wide().collect();
    let b: Vec<_> = b.as_os_str().encode_wide().collect();
    // SAFETY: initialized UTF-16 buffers with explicit lengths, ordinal OS comparison.
    unsafe {
        windows_sys::Win32::Globalization::CompareStringOrdinal(
            a.as_ptr(),
            a.len() as i32,
            b.as_ptr(),
            b.len() as i32,
            1,
        ) == 2
    }
}

pub fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let command = match args.as_slice() {
        [one] if one == "session" => None,
        [one] if one == "prepared" => Some(0u8),
        [one] if one == "commit" => Some(1),
        [one] if one == "uninstall" => Some(2),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "maintenance accepts one fixed verb",
            ));
        }
    };
    let _image = ProtectedInstallation::open(InstalledFile::Maintenance)?;
    // SAFETY: OS query only. Pin current image and elevated token before any effect.
    let process = peer::Process::open(unsafe { GetCurrentProcessId() })?;
    let _own_image = process.maintenance()?;
    let parent = peer::current_parent()?;
    // Current-thread runtime is required: a Windows mutex must be released by
    // the same thread that acquired it, including all error and timeout paths.
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        match command {
            None => session(parent).await,
            Some(command) => client(parent, command).await,
        }
    })
}
fn server(first: bool) -> io::Result<NamedPipeServer> {
    let sddl = wide("O:SYG:SYD:P(A;;GA;;;SY)(A;;GA;;;BA)");
    let mut descriptor = ptr::null_mut();
    // SAFETY: constant SDDL and initialized descriptor pointer.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    // SAFETY: no inheritance; Windows copies the live descriptor before returning.
    let result = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .in_buffer_size(16)
            .out_buffer_size(16)
            .create_with_security_attributes_raw(
                PIPE,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )
    };
    // SAFETY: LocalAlloc returned this descriptor.
    unsafe { LocalFree(descriptor) };
    let pipe = result?;
    Ok(pipe)
}
fn verify_peer(
    pipe: HANDLE,
    parent: &peer::Process,
    is_client: bool,
) -> io::Result<(peer::Process, ProtectedInstallation)> {
    let mut pid = 0;
    let ok = if is_client {
        // SAFETY: live connected pipe and initialized PID output.
        unsafe { GetNamedPipeServerProcessId(pipe, &mut pid) }
    } else {
        // SAFETY: live connected pipe and initialized PID output.
        unsafe { GetNamedPipeClientProcessId(pipe, &mut pid) }
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let process = peer::Process::open(pid)?;
    process.child_of(parent)?;
    let image = process.maintenance()?;
    Ok((process, image))
}
async fn session(parent: peer::Process) -> io::Result<()> {
    let mut pipe = server(true)?;
    let mut maintenance = Maintenance::new(machine::NativeMachine::new()?);
    let prepared = maintenance.prepare();
    let prepared_code = match &prepared {
        Ok(()) => 0,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => 2,
        Err(_) => 1,
    };
    let deadline = Instant::now() + SESSION_TIMEOUT;
    loop {
        tokio::select! {
            connection=pipe.connect()=>connection?,
            _=tokio::time::sleep(Duration::from_secs(1))=> {
                if !parent.alive() || Instant::now()>=deadline { return Err(io::Error::new(io::ErrorKind::TimedOut,"installer ended before commit")); }
                continue;
            }
        }
        let peer = verify_peer(pipe.as_raw_handle(), &parent, false);
        if let Ok((_peer, _image)) = peer {
            let command = tokio::time::timeout(Duration::from_secs(5), pipe.read_u8()).await;
            if let Ok(Ok(command)) = command {
                if !parent.alive() {
                    return Err(denied());
                }
                let result = match command {
                    0 => Ok(prepared_code),
                    1 if prepared_code == 0 => maintenance.commit().map(|()| 0),
                    2 if prepared_code == 0 => maintenance.uninstall().map(|()| 0),
                    _ => Err(denied()),
                };
                let code = match &result {
                    Ok(code) => *code,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => 2,
                    Err(_) => 1,
                };
                tokio::time::timeout(Duration::from_secs(5), pipe.write_u8(code))
                    .await
                    .map_err(|_error| denied())??;
                if command != 0 || prepared_code != 0 {
                    return result.map(|_| ());
                }
            }
        }
        // Keep the first instance alive until a replacement exists, so an
        // unprivileged process cannot seize the name between installer commands.
        let next = server(false)?;
        drop(pipe);
        pipe = next;
    }
}
fn connect(parent: &peer::Process) -> io::Result<NamedPipeClient> {
    let name = wide(PIPE);
    // SAFETY: fixed local-only pipe; identification SQOS prevents impersonation
    // of our elevated token by a server. Never asks for CREATE_PIPE_INSTANCE.
    let handle = own(unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | 0x0010_0000,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            ptr::null_mut(),
        )
    })?;
    let (_server, _image) = verify_peer(handle.as_raw_handle(), parent, true)?;
    // SAFETY: transfer one live overlapped pipe handle to Tokio exactly once.
    unsafe { NamedPipeClient::from_raw_handle(handle.into_raw_handle()) }
}
async fn client(parent: peer::Process, command: u8) -> io::Result<()> {
    tokio::time::timeout(IO_TIMEOUT, async {
        let mut pipe = loop {
            match connect(&parent) {
                Ok(pipe) => break pipe,
                Err(error) if matches!(error.raw_os_error(), Some(2 | 231)) && parent.alive() => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                Err(error) => return Err(error),
            }
        };
        pipe.write_u8(command).await?;
        match pipe.read_u8().await? {
            0 => Ok(()),
            2 => Err(io::Error::new(io::ErrorKind::WouldBlock, "restart required before repair")),
            _ => Err(denied()),
        }
    })
    .await
    .map_err(|_error| io::Error::new(io::ErrorKind::TimedOut, "maintenance session unavailable"))?
}
