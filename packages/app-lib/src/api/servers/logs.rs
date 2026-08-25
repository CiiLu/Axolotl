//! Console output buffering and streaming for running servers.

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::Result;
use crate::event::ServerPayloadType;
use crate::event::emit::emit_server;
use crate::state::{clear_log_buffer, push_log_line};

pub async fn get_log_buffer(server_id: &str) -> Result<Vec<String>> {
    Ok(crate::state::get_log_buffer(server_id))
}

pub async fn clear_log(server_id: &str) -> Result<()> {
    clear_log_buffer(server_id);
    Ok(())
}

pub(super) async fn stream_server_output(
	server_id: String,
	reader: impl tokio::io::AsyncRead + Unpin,
) {
	let mut buf_reader = BufReader::new(reader);
	let mut line = String::new();
	let mut jna_hint_emitted = false;
	loop {
		line.clear();
		match buf_reader.read_line(&mut line).await {
			Ok(0) | Err(_) => break,
			Ok(_) => {
				let trimmed = line.trim_end_matches(['\r', '\n']);
				let cleaned = strip_ansi(trimmed);
				if cleaned.is_empty() {
					continue;
				}
				push_log_line(&server_id, cleaned.clone());
				emit_server(&server_id, ServerPayloadType::Log { line: cleaned })
					.await
					.ok();
				if !jna_hint_emitted && is_jna_macos_assertion(trimmed) {
					jna_hint_emitted = true;
					for hint in JNA_CRASH_HINT_LINES {
						push_log_line(&server_id, hint.to_string());
						emit_server(&server_id, ServerPayloadType::Log { line: hint.to_string() })
							.await
							.ok();
					}
				}
			}
		}
	}
}

/// Matches the native abort of the known JNA (< 5.13.0) macOS bug (JNA issue
/// #1452): a failed library load overflows JNA's fixed error buffer and the
/// JVM dies with SIGABRT before any Java-level exception can be reported.
fn is_jna_macos_assertion(line: &str) -> bool {
	line.contains("Assertion failed:")
		&& line.contains("snprintf() output has been truncated")
		&& line.contains("dispatch.c")
}

const JNA_CRASH_HINT_LINES: [&str; 3] = [
	"[Axolotl] This crash matches a known JNA bug on macOS (java-native-access#1452):",
	"[Axolotl] mods bundling JNA below 5.13.0 abort when a native library fails to load.",
	"[Axolotl] Update or remove the affected mod, or ask the modpack author to bump JNA to 5.13.0+.",
];

/// Removes ANSI escape sequences (SGR colors, cursor control, OSC titles) that
/// servers emit when they assume an interactive terminal is attached.
fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((_, character)) = chars.next() {
        if character != '\u{1b}' {
            output.push(character);
            continue;
        }
        match chars.peek().map(|&(_, c)| c) {
            // CSI sequence: parameter bytes, then a final byte in @..~
            Some('[') => {
                chars.next();
                for (_, c) in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            // OSC sequence: terminated by BEL or ST (ESC \)
            Some(']') => {
                chars.next();
                let mut saw_escape = false;
                for (_, c) in chars.by_ref() {
                    if c == '\u{7}' || (saw_escape && c == '\\') {
                        break;
                    }
                    saw_escape = c == '\u{1b}';
                }
            }
            // Stray escape byte without a recognized sequence
            _ => {}
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_escape_sequences_from_server_output() {
        let line = "[16:02:30 INFO]: \u{1b}[38;2;255;170;0m/mspt: \u{1b}[38;2;255;255;255mView server tick times\u{1b}[0m";
        assert_eq!(
            strip_ansi(line),
            "[16:02:30 INFO]: /mspt: View server tick times"
        );

		assert_eq!(strip_ansi("\u{1b}]0;Server console\u{7}ready"), "ready");
		assert_eq!(strip_ansi("\u{1b}]0;Server console\u{1b}\\done"), "done");
		assert_eq!(strip_ansi("plain text stays"), "plain text stays");
		assert_eq!(strip_ansi("h\u{e9}llo \u{1b}[31mred"), "h\u{e9}llo red");
	}

	#[test]
	fn detects_jna_macos_assertion() {
		let line = "Assertion failed: (count <= len && \"snprintf() output has been truncated\"), function LOAD_ERROR, file dispatch.c, line 74.";
		assert!(is_jna_macos_assertion(line));
		assert!(!is_jna_macos_assertion(
			"Assertion failed: something else, file other.c, line 1."
		));
		assert!(!is_jna_macos_assertion("regular log output"));
	}
}
