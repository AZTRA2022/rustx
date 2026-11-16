//! HTTPS, through the system's own client.
//!
//! WinHTTP rather than a crate: this exists for the updater, and an
//! updater that brought its own TLS stack would be a second place for
//! certificates to be wrong. The system's client already has the
//! machine's trust store, the machine's proxy settings — which is what
//! makes this work inside a company — and the machine's security
//! updates.
//!
//! It is deliberately small: a `GET`, redirects followed, the body in
//! memory or written to a file as it arrives. That is what reading a
//! release page and downloading an archive need, and nothing else in
//! this framework makes an HTTP request.

use std::io::Write;
use std::path::Path;

use windows::Win32::Networking::WinHttp::{
    URL_COMPONENTS, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_OPEN_REQUEST_FLAGS, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    WinHttpCloseHandle, WinHttpConnect, WinHttpCrackUrl, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::PCWSTR;

use crate::error::OsError;

/// What a request is allowed to take, in milliseconds. A window that
/// checks for an update must not hang on a captive portal.
const TIMEOUT: i32 = 20_000;

/// The whole body of a `GET`, in memory.
///
/// For the release description, which is a few kilobytes of JSON. An
/// archive goes to [`download`] instead: a 300 MB `Vec<u8>` in a window
/// process is not a thing to do when a file will do.
pub fn get(url: &str, agent: &str) -> Result<Vec<u8>, OsError> {
    let mut out = Vec::new();
    request(url, agent, &mut |piece| {
        out.extend_from_slice(piece);
        Ok(())
    })?;
    Ok(out)
}

/// The same, written to a file as it arrives, with the number of bytes
/// handed to `progress` as they land.
///
/// The progress callback is what lets a page draw a bar. It is called
/// on the thread that called this, which for an update is a worker
/// thread -- so what it does with the number is post it, like anything
/// else that crosses to the frame.
pub fn download(
    url: &str,
    agent: &str,
    into: &Path,
    progress: &mut dyn FnMut(u64),
) -> Result<u64, OsError> {
    let handle = std::fs::File::create(into).map_err(|error| {
        OsError::Platform(format!("{} could not be written ({error})", into.display()))
    })?;
    let mut file = std::io::BufWriter::new(handle);
    let mut total = 0_u64;
    request(url, agent, &mut |piece| {
        file.write_all(piece).map_err(|error| {
            OsError::Platform(format!("{} could not be written ({error})", into.display()))
        })?;
        total += piece.len() as u64;
        progress(total);
        Ok(())
    })?;
    file.flush().map_err(|error| {
        OsError::Platform(format!(
            "{} could not be finished ({error})",
            into.display()
        ))
    })?;
    Ok(total)
}

/// One `GET`, with each piece of the body handed to `take` as it
/// arrives.
fn request(
    url: &str,
    agent: &str,
    take: &mut dyn FnMut(&[u8]) -> Result<(), OsError>,
) -> Result<(), OsError> {
    let wide_url = wide(url);
    let mut host = [0_u16; 256];
    let mut path = [0_u16; 2048];
    let mut parts = URL_COMPONENTS {
        dwStructSize: std::mem::size_of::<URL_COMPONENTS>() as u32,
        lpszHostName: windows::core::PWSTR(host.as_mut_ptr()),
        dwHostNameLength: host.len() as u32,
        lpszUrlPath: windows::core::PWSTR(path.as_mut_ptr()),
        dwUrlPathLength: path.len() as u32,
        ..Default::default()
    };

    // SAFETY: `wide_url` is nul-terminated and outlives the call;
    // `parts` points at the two local buffers above, whose lengths are
    // given in the same struct, and WinHTTP writes inside them and no
    // further. The struct's `dwStructSize` is set, which is the field
    // that tells the system which version of it this is.
    unsafe { WinHttpCrackUrl(&wide_url, 0, &mut parts) }.map_err(|error| {
        OsError::Platform(format!("`{url}` is not a URL WinHTTP accepts ({error})"))
    })?;
    let port = parts.nPort;
    let secure = port == 443;
    let host_name = from_wide(&host, parts.dwHostNameLength as usize);
    let url_path = from_wide(&path, parts.dwUrlPathLength as usize);

    let agent = wide(agent);
    // SAFETY: the agent string is nul-terminated and outlives the call.
    // `WINHTTP_NO_PROXY_NAME` with the automatic access type is the
    // documented pair for "use whatever this machine is configured
    // with", which is what makes this work behind a company proxy.
    let session = unsafe {
        WinHttpOpen(
            PCWSTR(agent.as_ptr()),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        )
    };
    if session.is_null() {
        return Err(OsError::Platform(
            "WinHTTP could not be opened; this machine has no HTTP client the system \
             will lend"
                .to_string(),
        ));
    }
    let session = Handle(session);

    // SAFETY: the handle is the live session above; the five numbers
    // are milliseconds and nothing is read through a pointer.
    unsafe {
        let _ = WinHttpSetTimeouts(session.0, TIMEOUT, TIMEOUT, TIMEOUT, TIMEOUT);
    }

    let wide_host = wide(&host_name);
    // SAFETY: `wide_host` is nul-terminated and outlives the call, and
    // the session handle is live for the whole of this function -- the
    // connection it returns is a child of it and is closed first, by
    // the drop order of these locals.
    let connection = unsafe { WinHttpConnect(session.0, PCWSTR(wide_host.as_ptr()), port, 0) };
    if connection.is_null() {
        return Err(OsError::Platform(format!(
            "{host_name} could not be reached"
        )));
    }
    let connection = Handle(connection);

    let verb = wide("GET");
    let wide_path = wide(&url_path);
    // SAFETY: both strings are nul-terminated locals that outlive the
    // call. The null accept-types pointer is the documented "anything";
    // `WINHTTP_FLAG_SECURE` is what makes this HTTPS, and it is set
    // from the port the URL itself named.
    let request = unsafe {
        WinHttpOpenRequest(
            connection.0,
            PCWSTR(verb.as_ptr()),
            PCWSTR(wide_path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null_mut(),
            if secure {
                WINHTTP_FLAG_SECURE
            } else {
                WINHTTP_OPEN_REQUEST_FLAGS(0)
            },
        )
    };
    if request.is_null() {
        return Err(OsError::Platform(format!(
            "the request for {url} could not be made"
        )));
    }
    let request = Handle(request);

    // SAFETY: the request handle is live. No extra headers and no body,
    // so every pointer here is the documented null and every length is
    // zero.
    unsafe { WinHttpSendRequest(request.0, None, None, 0, 0, 0) }
        .map_err(|error| OsError::Platform(format!("{url} could not be asked for ({error})")))?;

    // SAFETY: the request was sent; this waits for the headers. The
    // reserved parameter is the documented null.
    unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
        .map_err(|error| OsError::Platform(format!("{url} did not answer ({error})")))?;

    let mut status = 0_u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: the two out-parameters are live locals, and `size` says
    // how big the first one is -- four bytes, because
    // `WINHTTP_QUERY_FLAG_NUMBER` asks for the status as a number.
    unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut status as *mut u32 as *mut _),
            &mut size,
            std::ptr::null_mut(),
        )
    }
    .map_err(|error| OsError::Platform(format!("{url} answered nothing readable ({error})")))?;

    if !(200..300).contains(&status) {
        return Err(OsError::Platform(format!(
            "{url} answered {status}. {}",
            match status {
                404 => "There is no such release, or the repository is private.",
                403 =>
                    "The request was refused -- GitHub rate-limits anonymous \
                        callers by address.",
                _ => "Nothing was downloaded.",
            }
        )));
    }

    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let mut read = 0_u32;
        // SAFETY: `buffer` is a live allocation of the length handed
        // over, and `read` is a live local the call fills in. WinHTTP
        // writes at most that many bytes.
        unsafe {
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut read,
            )
        }
        .map_err(|error| OsError::Platform(format!("{url} stopped mid-answer ({error})")))?;
        if read == 0 {
            break;
        }
        take(&buffer[..read as usize])?;
    }
    Ok(())
}

/// A WinHTTP handle that closes itself.
struct Handle(*mut std::ffi::c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the handle was returned by WinHTTP and is closed
            // exactly once, here, because this type owns it and is not
            // `Clone`.
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buffer: &[u16], length: usize) -> String {
    String::from_utf16_lossy(&buffer[..length.min(buffer.len())])
}
