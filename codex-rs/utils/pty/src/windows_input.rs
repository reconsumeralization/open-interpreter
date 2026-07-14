/// Stateful normalizer for bytes written to a Windows pseudoconsole.
///
/// ConPTY accepts UTF-8 input, but an Enter key is represented by a carriage
/// return on Windows. This converts line feeds and collapses existing CRLF
/// sequences, including when the two bytes arrive in separate writes, so each
/// requested newline submits exactly one line. Backspace is encoded as DEL,
/// which ConPTY translates to `VK_BACK`. Ctrl-C is encoded as Win32 input-mode
/// key records because the ConPTY instance requests that mode; a raw ETX byte
/// does not interrupt foreground console processes on current Windows builds.
/// All other bytes, including UTF-8 and terminal control characters, pass
/// through unchanged.
#[derive(Default)]
pub struct WindowsTtyInputNormalizer {
    previous_was_cr: bool,
}

impl WindowsTtyInputNormalizer {
    pub fn normalize(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut normalized = Vec::with_capacity(bytes.len());
        for &byte in bytes {
            match byte {
                b'\x08' => normalized.push(b'\x7f'),
                b'\x03' => normalized.extend_from_slice(b"\x1b[67;0;3;1;8;1_\x1b[67;0;3;0;8;1_"),
                b'\n' => {
                    if !self.previous_was_cr {
                        normalized.push(b'\r');
                    }
                }
                _ => normalized.push(byte),
            }
            self.previous_was_cr = byte == b'\r';
        }
        normalized
    }
}

#[cfg(test)]
#[path = "windows_input_tests.rs"]
mod tests;
