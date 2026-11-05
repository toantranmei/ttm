//! Spinners and byte progress (indicatif), drawn on stderr and only for a person at a terminal (hidden when piped,
//! with `--plain` or `--json`).

use crate::Ui;
use crate::callout::Callout;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::time::Duration;

/// A spinner with a message; [`Spinner::finish`] replaces it with a callout on stdout.
pub struct Spinner {
    bar: Option<ProgressBar>,
    ui: Ui,
}

/// A byte progress bar (downloads); [`Progress::finish`] replaces it with a callout on stdout.
pub struct Progress {
    bar: Option<ProgressBar>,
    ui: Ui,
}

const TICKS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ";

impl Ui {
    /// Starts a spinner on stderr (nothing is drawn when the output is not for a person at a terminal).
    pub fn spinner(&self, message: impl Into<String>) -> Spinner {
        let bar = self.mode().animated().then(|| {
            let bar = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
            let template = if self.mode().colored() {
                "{spinner:.cyan} {msg}"
            } else {
                "{spinner} {msg}"
            };
            let style = ProgressStyle::with_template(template)
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_chars(TICKS);
            bar.set_style(style);
            bar.set_message(message.into());
            bar.enable_steady_tick(Duration::from_millis(100));
            bar
        });
        Spinner {
            bar,
            ui: self.clone(),
        }
    }

    /// Starts a byte progress bar of `total` bytes on stderr (hidden like [`Ui::spinner`]).
    pub fn bytes(&self, total: u64, message: impl Into<String>) -> Progress {
        let bar = self.mode().animated().then(|| {
            let bar = ProgressBar::with_draw_target(Some(total), ProgressDrawTarget::stderr());
            let style = ProgressStyle::with_template("{msg} {bar:30} {bytes}/{total_bytes} {eta}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("━╸ ");
            bar.set_style(style);
            bar.set_message(message.into());
            bar
        });
        Progress {
            bar,
            ui: self.clone(),
        }
    }
}

impl Spinner {
    pub fn set_message(&self, message: impl Into<String>) {
        if let Some(bar) = &self.bar {
            bar.set_message(message.into());
        }
    }

    /// Prints a line on stdout without breaking the spinner.
    pub fn line(&self, text: &str) {
        match &self.bar {
            Some(bar) => bar.suspend(|| self.ui.line(text)),
            None => self.ui.line(text),
        }
    }

    /// Removes the spinner.
    pub fn clear(mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }

    /// Removes the spinner and prints `callout` on stdout.
    pub fn finish(self, callout: Callout) {
        let ui = self.ui.clone();
        self.clear();
        ui.print(&callout);
    }

    /// Removes the spinner and prints `callout` on stderr (status lines that are not the result).
    pub fn finish_err(self, callout: Callout) {
        let ui = self.ui.clone();
        self.clear();
        ui.eprint(&callout);
    }

    /// Whether anything is drawn.
    pub fn visible(&self) -> bool {
        self.bar.is_some()
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }
}

impl Progress {
    pub fn inc(&self, bytes: u64) {
        if let Some(bar) = &self.bar {
            bar.inc(bytes);
        }
    }

    pub fn set_position(&self, bytes: u64) {
        if let Some(bar) = &self.bar {
            bar.set_position(bytes);
        }
    }

    pub fn clear(mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }

    pub fn finish(self, callout: Callout) {
        let ui = self.ui.clone();
        self.clear();
        ui.print(&callout);
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mode::{ColorLevel, Mode};
    use crate::{Callout, Ui};

    #[test]
    fn hidden_without_a_terminal() {
        let (ui, buf) = Ui::buffered(Mode::fixed(ColorLevel::None, false, 80));
        let s = ui.spinner("Waiting");
        assert!(!s.visible());
        s.line("event");
        s.finish(Callout::success("Done"));
        assert_eq!(buf.stdout(), "event\n✓ Done\n");
        assert_eq!(buf.stderr(), "");
    }
}
