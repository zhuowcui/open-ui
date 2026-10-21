//! Win32 Unicode text clipboard for the Windows desktop runtime.

use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

/// The clipboard is a process-global Win32 resource; this owns nothing.
pub(crate) struct WindowsClipboard;

/// Closes the clipboard when an opened scope ends, including early returns.
struct OpenScope;

impl OpenScope {
    fn open() -> Option<Self> {
        // SAFETY: a null owner associates the clipboard with this task.
        (unsafe { OpenClipboard(std::ptr::null_mut()) } != 0).then_some(Self)
    }
}

impl Drop for OpenScope {
    fn drop(&mut self) {
        // SAFETY: this scope opened the clipboard successfully.
        unsafe { CloseClipboard() };
    }
}

impl WindowsClipboard {
    pub(crate) fn load(&self) -> Result<String, String> {
        // SAFETY: querying format availability has no preconditions.
        if unsafe { IsClipboardFormatAvailable(u32::from(CF_UNICODETEXT)) } == 0 {
            return Err("the clipboard holds no Unicode text".into());
        }
        let _scope = OpenScope::open().ok_or("the clipboard is in use")?;
        // SAFETY: the clipboard is open; the returned handle stays owned by it.
        let handle: HANDLE = unsafe { GetClipboardData(u32::from(CF_UNICODETEXT)) };
        if handle.is_null() {
            return Err("the clipboard text could not be read".into());
        }
        // SAFETY: CF_UNICODETEXT data is a movable global memory object.
        let data = unsafe { GlobalLock(handle as HGLOBAL) } as *const u16;
        if data.is_null() {
            return Err("the clipboard text could not be locked".into());
        }
        let mut length = 0;
        // SAFETY: CF_UNICODETEXT is NUL-terminated UTF-16 within the locked object.
        while unsafe { *data.add(length) } != 0 {
            length += 1;
        }
        let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(data, length) });
        // SAFETY: balances the GlobalLock above.
        unsafe { GlobalUnlock(handle as HGLOBAL) };
        Ok(text)
    }

    pub(crate) fn store(&self, text: String) {
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let Some(_scope) = OpenScope::open() else {
            return;
        };
        // SAFETY: the clipboard is open and owned by this task.
        unsafe { EmptyClipboard() };
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        // SAFETY: allocates a movable object that the clipboard may own.
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
        if memory.is_null() {
            return;
        }
        // SAFETY: `memory` is a fresh movable object of `bytes` bytes.
        let target = unsafe { GlobalLock(memory) } as *mut u16;
        if target.is_null() {
            unsafe { GlobalFree(memory) };
            return;
        }
        // SAFETY: both buffers hold `utf16.len()` code units and do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), target, utf16.len());
            GlobalUnlock(memory);
        }
        // SAFETY: on success the clipboard owns `memory`; otherwise free it.
        if unsafe { SetClipboardData(u32::from(CF_UNICODETEXT), memory as HANDLE) }.is_null() {
            unsafe { GlobalFree(memory) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsClipboard;

    #[test]
    fn unicode_text_round_trips_and_the_previous_text_is_restored() {
        let clipboard = WindowsClipboard;
        let previous = clipboard.load().ok();
        let text = "Open UI ✓ 日本語 \u{1F600}".to_string();
        clipboard.store(text.clone());
        let loaded = clipboard.load();
        if let Some(previous) = previous {
            clipboard.store(previous);
        }
        assert_eq!(loaded, Ok(text));
    }
}
