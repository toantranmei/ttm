//! `ttm-ui`: the one place where ttm prints for people.
//!
//! Every command renders through [`Ui`] with the components of this crate ([`Header`], [`Table`], [`Status`],
//! [`Badge`], [`KeyValue`], [`Callout`], [`Diff`], [`ErrorView`], spinners), so the output looks the same everywhere:
//! the result on the first line, details, then `Next: <command>`.
//!
//! Modes ([`Mode`]): brand colours (truecolor → 256 → 16) on a terminal; no colour when piped, with `NO_COLOR` or
//! `TERM=dumb`; `--plain`/`TTM_PLAIN=1` drops frames, glyphs and emoji; `--json` keeps stdout for JSON only. Width:
//! `TTM_COLUMNS` > `COLUMNS` > terminal > 80, clamped to 40..=200.

pub mod badge;
pub mod callout;
pub mod diff;
pub mod error;
pub mod header;
pub mod kv;
pub mod mode;
pub mod progress;
pub mod status;
pub mod table;
pub mod theme;
pub mod width;

pub use badge::Badge;
pub use callout::{Callout, Kind as CalloutKind, next_line};
pub use diff::{Change, ChangeKind, Diff};
pub use error::{ErrorView, UsageError, hint};
pub use header::Header;
pub use kv::KeyValue;
pub use mode::{ColorLevel, Mode, Probe};
pub use progress::{Progress, Spinner};
pub use status::{Level, Status};
pub use table::{Cell, Col, Table};
pub use theme::{Token, paint};
pub use ttm_api::vocab;
pub use width::{Align, display_width, pad, strip_ansi, truncate};

use std::io::Write;
use std::sync::{Arc, Mutex};

/// Something that can draw itself for a [`Mode`]. The text has no trailing newline.
pub trait Render {
    fn render(&self, mode: &Mode) -> String;
}

impl Render for str {
    fn render(&self, _mode: &Mode) -> String {
        self.to_string()
    }
}

impl Render for String {
    fn render(&self, _mode: &Mode) -> String {
        self.clone()
    }
}

impl<T: Render + ?Sized> Render for &T {
    fn render(&self, mode: &Mode) -> String {
        (**self).render(mode)
    }
}

type Sink = Mutex<Box<dyn Write + Send>>;

struct Inner {
    mode: Mode,
    out: Sink,
    err: Sink,
}

/// Output handle: the mode plus stdout and stderr. Cheap to clone (shared).
#[derive(Clone)]
pub struct Ui {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Ui {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ui").field("mode", &self.inner.mode).finish()
    }
}

/// What a buffered [`Ui`] wrote, for tests.
#[derive(Clone, Default)]
pub struct SharedBuf {
    out: Arc<Mutex<Vec<u8>>>,
    err: Arc<Mutex<Vec<u8>>>,
}

impl SharedBuf {
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.out.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.err.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
    }
}

struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Ui {
    /// The real stdout/stderr, mode detected from the environment.
    pub fn detect(plain_flag: bool, json_flag: bool) -> Ui {
        Ui::stdio(Mode::detect(plain_flag, json_flag))
    }

    /// The real stdout/stderr with a given mode. `anstream` strips stray escape codes when the mode has no colour
    /// and translates them for old Windows consoles.
    pub fn stdio(mode: Mode) -> Ui {
        let choice = if mode.colored() {
            anstream::ColorChoice::Always
        } else {
            anstream::ColorChoice::Never
        };
        let out = anstream::AutoStream::new(std::io::stdout(), choice);
        let err = anstream::AutoStream::new(std::io::stderr(), choice);
        Ui::with_writers(mode, Box::new(out), Box::new(err))
    }

    pub fn with_writers(mode: Mode, out: Box<dyn Write + Send>, err: Box<dyn Write + Send>) -> Ui {
        Ui {
            inner: Arc::new(Inner {
                mode,
                out: Mutex::new(out),
                err: Mutex::new(err),
            }),
        }
    }

    /// A Ui writing into memory (tests).
    pub fn buffered(mode: Mode) -> (Ui, SharedBuf) {
        let buf = SharedBuf::default();
        let ui = Ui::with_writers(
            mode,
            Box::new(BufWriter(buf.out.clone())),
            Box::new(BufWriter(buf.err.clone())),
        );
        (ui, buf)
    }

    pub fn mode(&self) -> &Mode {
        &self.inner.mode
    }

    fn write(sink: &Sink, text: &str) {
        let mut w = sink.lock().unwrap_or_else(|e| e.into_inner());
        let _ = w.write_all(text.as_bytes());
        let _ = w.flush();
    }

    /// One line on stdout (dropped with `--json`: stdout is for JSON only).
    pub fn line(&self, text: &str) {
        if self.inner.mode.json {
            return;
        }
        Ui::write(&self.inner.out, &format!("{text}\n"));
    }

    /// An empty line on stdout.
    pub fn blank(&self) {
        self.line("");
    }

    /// A component on stdout.
    pub fn print<R: Render + ?Sized>(&self, item: &R) {
        let text = item.render(&self.inner.mode);
        if !text.is_empty() {
            self.line(&text);
        }
    }

    /// Text on stdout exactly as given (Markdown to pipe, JSON). Not dropped with `--json`.
    pub fn raw(&self, text: &str) {
        Ui::write(&self.inner.out, text);
    }

    /// One line on stderr.
    pub fn eline(&self, text: &str) {
        Ui::write(&self.inner.err, &format!("{text}\n"));
    }

    /// A component on stderr.
    pub fn eprint<R: Render + ?Sized>(&self, item: &R) {
        let text = item.render(&self.inner.mode);
        if !text.is_empty() {
            self.eline(&text);
        }
    }

    /// `text` in the colour of `token` for this mode.
    pub fn paint(&self, token: Token, text: &str) -> String {
        theme::paint(&self.inner.mode, token, text)
    }

    /// A component rendered for this mode.
    pub fn render<R: Render + ?Sized>(&self, item: &R) -> String {
        item.render(&self.inner.mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffered_ui_and_json_mode() {
        let (ui, buf) = Ui::buffered(Mode::fixed(ColorLevel::None, false, 80));
        ui.line("hello");
        ui.blank();
        ui.eline("oops");
        assert_eq!(buf.stdout(), "hello\n\n");
        assert_eq!(buf.stderr(), "oops\n");

        let mut mode = Mode::fixed(ColorLevel::None, false, 80);
        mode.json = true;
        let (ui, buf) = Ui::buffered(mode);
        ui.line("human");
        ui.raw("{}\n");
        ui.eline("still on stderr");
        assert_eq!(buf.stdout(), "{}\n");
        assert_eq!(buf.stderr(), "still on stderr\n");
    }
}
