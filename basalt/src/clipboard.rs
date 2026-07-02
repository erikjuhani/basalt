//! Copy text to the host clipboard without a system clipboard dependency.
//!
//! Two best-effort paths run together: a native clipboard utility (`pbcopy`,
//! `wl-copy`, `xclip`, `xsel`, or `clip`) which is reliable locally, and an
//! OSC 52 terminal escape which travels through SSH and tmux (with
//! `set-clipboard on`). Terminals that disable OSC 52 ignore it silently, so on
//! a local session the native utility is what actually fills the clipboard.

use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Copies `text` to the clipboard. Reports success when either the native
/// utility ran or the OSC 52 escape was written.
pub fn copy(text: &str) -> io::Result<()> {
    let native = native_copy(text);
    let osc52 = osc52(text);
    if native {
        Ok(())
    } else {
        osc52
    }
}

/// Writes `text` to the terminal clipboard using `OSC 52 ; c ; <base64> BEL`.
fn osc52(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{}\x07", base64(text.as_bytes()))?;
    stdout.flush()
}

/// Pipes `text` into the first available platform clipboard utility. Returns
/// whether one ran successfully.
fn native_copy(text: &str) -> bool {
    clipboard_commands()
        .iter()
        .any(|(command, args)| pipe_to(command, args, text).is_ok())
}

fn clipboard_commands() -> &'static [(&'static str, &'static [&'static str])] {
    if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["-b"]),
        ]
    }
}

fn pipe_to(command: &str, args: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("clipboard command has no stdin"))?
        .write_all(text.as_bytes())?;

    child.wait()?;
    Ok(())
}

/// Standard base64 (RFC 4648) with `=` padding.
fn base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    input
        .chunks(3)
        .flat_map(|chunk| {
            let [a, b, c] = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let triple = (a as u32) << 16 | (b as u32) << 8 | c as u32;
            let indices = [
                triple >> 18 & 0x3f,
                triple >> 12 & 0x3f,
                triple >> 6 & 0x3f,
                triple & 0x3f,
            ];
            indices.into_iter().enumerate().map(move |(i, index)| {
                // Pad the slots that have no source byte behind them.
                if i > chunk.len() {
                    b'='
                } else {
                    ALPHABET[index as usize]
                }
            })
        })
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn test_base64_matches_known_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
            ("hello world", "aGVsbG8gd29ybGQ="),
        ];

        cases
            .into_iter()
            .for_each(|(input, expected)| assert_eq!(base64(input.as_bytes()), expected));
    }
}
