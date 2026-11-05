//! Wizards where every question has a flag.
//!
//! Each [`Ask`] names the flag that answers it. A value from the flag is checked and used without asking; with
//! `--yes` the default is taken (or the command fails with `missing_flag`); without a terminal and without `--yes`
//! the command fails with `missing_flag` ("pass --workflow or run in a terminal"); otherwise the question is asked
//! with `inquire` in the brand colours. Afterwards [`Wizard::print_equivalent`] prints the same command with every
//! flag filled in, to paste into a script.

use crate::Ui;
use crate::error::UsageError;
use crate::mode::{ColorLevel, Mode};
use crate::theme::Token;
use anyhow::{Result, anyhow};
use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

/// How a question is put to a person. [`InquireTerm`] is the terminal; [`ScriptedTerm`] answers from a list (tests).
pub trait Term {
    fn text(&mut self, label: &str, default: Option<&str>, help: Option<&str>) -> Result<String>;
    fn select(
        &mut self,
        label: &str,
        options: &[String],
        default: Option<usize>,
        help: Option<&str>,
    ) -> Result<usize>;
    fn multi_select(
        &mut self,
        label: &str,
        options: &[String],
        defaults: &[usize],
        help: Option<&str>,
    ) -> Result<Vec<usize>>;
    fn confirm(&mut self, label: &str, default: bool) -> Result<bool>;
    /// Hidden input (never echoed, never a flag value).
    fn secret(&mut self, label: &str) -> Result<String>;
}

/// The terminal, through `inquire`, styled with the brand tokens.
pub struct InquireTerm {
    render: inquire::ui::RenderConfig<'static>,
}

fn inquire_color(token: Token, level: ColorLevel) -> Option<inquire::ui::Color> {
    use inquire::ui::Color;
    match level {
        ColorLevel::None => None,
        ColorLevel::TrueColor => match token {
            Token::Muted => Some(Color::DarkGrey),
            _ => token.rgb().map(|c| Color::Rgb {
                r: c.0,
                g: c.1,
                b: c.2,
            }),
        },
        ColorLevel::Ansi256 => match token {
            Token::Muted => Some(Color::DarkGrey),
            _ => token
                .rgb()
                .map(|c| Color::AnsiValue(anstyle_lossy::rgb_to_xterm(c).0)),
        },
        ColorLevel::Ansi16 => match token {
            Token::Teal => Some(Color::DarkCyan),
            Token::Yellow => Some(Color::DarkYellow),
            Token::Coral => Some(Color::DarkRed),
            Token::Purple => Some(Color::DarkMagenta),
            Token::Blue => Some(Color::DarkBlue),
            Token::Gold => Some(Color::LightYellow),
            Token::Muted => Some(Color::DarkGrey),
            Token::Ink => None,
        },
    }
}

impl InquireTerm {
    /// `RenderConfig` for `mode`: question `Ink`, hints `Muted`, highlighted choice `Teal`, errors `Coral`.
    pub fn render_config(mode: &Mode) -> inquire::ui::RenderConfig<'static> {
        use inquire::ui::{Attributes, ErrorMessageRenderConfig, RenderConfig, StyleSheet, Styled};
        if !mode.colored() {
            let base = RenderConfig::empty();
            return if mode.plain {
                base
            } else {
                base.with_answered_prompt_prefix(Styled::new("✓"))
                    .with_highlighted_option_prefix(Styled::new("›"))
            };
        }
        let level = mode.color;
        let sheet = |token: Token| {
            let s = StyleSheet::new();
            match inquire_color(token, level) {
                Some(c) => s.with_fg(c),
                None => s,
            }
        };
        let styled = |text: &'static str, token: Token| {
            let s = Styled::new(text);
            match inquire_color(token, level) {
                Some(c) => s.with_fg(c),
                None => s,
            }
        };
        let mut config = RenderConfig::empty()
            .with_prompt_prefix(styled("?", Token::Teal))
            .with_answered_prompt_prefix(styled("✓", Token::Teal))
            .with_highlighted_option_prefix(styled("›", Token::Teal))
            .with_selected_option(Some(sheet(Token::Teal)))
            .with_selected_checkbox(styled("[x]", Token::Teal))
            .with_answer(sheet(Token::Teal))
            .with_help_message(sheet(Token::Muted))
            .with_default_value(sheet(Token::Muted))
            .with_canceled_prompt_indicator(styled("<canceled>", Token::Coral))
            .with_error_message(
                ErrorMessageRenderConfig::empty()
                    .with_prefix(styled("✕", Token::Coral))
                    .with_message(sheet(Token::Coral)),
            );
        config.prompt = StyleSheet::new().with_attr(Attributes::BOLD);
        config.placeholder = sheet(Token::Muted);
        config
    }

    pub fn for_mode(mode: &Mode) -> Self {
        Self {
            render: InquireTerm::render_config(mode),
        }
    }
}

fn canceled(err: inquire::InquireError) -> anyhow::Error {
    match err {
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow!("canceled")
        }
        inquire::InquireError::NotTTY => anyhow!("not a terminal"),
        other => anyhow!(other),
    }
}

impl Term for InquireTerm {
    fn text(&mut self, label: &str, default: Option<&str>, help: Option<&str>) -> Result<String> {
        let mut prompt = inquire::Text::new(label).with_render_config(self.render);
        if let Some(d) = default {
            prompt = prompt.with_default(d);
        }
        if let Some(h) = help {
            prompt = prompt.with_help_message(h);
        }
        prompt.prompt().map_err(canceled)
    }

    fn select(
        &mut self,
        label: &str,
        options: &[String],
        default: Option<usize>,
        help: Option<&str>,
    ) -> Result<usize> {
        let mut prompt = inquire::Select::new(label, options.to_vec()).with_render_config(self.render);
        if let Some(d) = default {
            prompt = prompt.with_starting_cursor(d);
        }
        if let Some(h) = help {
            prompt = prompt.with_help_message(h);
        }
        prompt.raw_prompt().map(|o| o.index).map_err(canceled)
    }

    fn multi_select(
        &mut self,
        label: &str,
        options: &[String],
        defaults: &[usize],
        help: Option<&str>,
    ) -> Result<Vec<usize>> {
        let mut prompt = inquire::MultiSelect::new(label, options.to_vec())
            .with_render_config(self.render)
            .with_default(defaults);
        if let Some(h) = help {
            prompt = prompt.with_help_message(h);
        }
        prompt
            .raw_prompt()
            .map(|chosen| chosen.into_iter().map(|o| o.index).collect())
            .map_err(canceled)
    }

    fn confirm(&mut self, label: &str, default: bool) -> Result<bool> {
        inquire::Confirm::new(label)
            .with_default(default)
            .with_render_config(self.render)
            .prompt()
            .map_err(canceled)
    }

    fn secret(&mut self, label: &str) -> Result<String> {
        inquire::Password::new(label)
            .without_confirmation()
            .with_render_config(self.render)
            .prompt()
            .map_err(canceled)
    }
}

/// One scripted answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    Select(usize),
    Multi(Vec<usize>),
    Confirm(bool),
    Secret(String),
}

/// Answers questions from a list and records the labels asked (tests).
#[derive(Debug, Clone, Default)]
pub struct ScriptedTerm {
    answers: Arc<Mutex<VecDeque<Answer>>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl ScriptedTerm {
    pub fn new(answers: impl IntoIterator<Item = Answer>) -> Self {
        Self {
            answers: Arc::new(Mutex::new(answers.into_iter().collect())),
            asked: Arc::default(),
        }
    }

    /// Labels of the questions asked so far (shared with clones).
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn next(&self, label: &str) -> Result<Answer> {
        self.asked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(label.to_string());
        self.answers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
            .ok_or_else(|| anyhow!("no scripted answer for `{label}`"))
    }
}

impl Term for ScriptedTerm {
    fn text(&mut self, label: &str, default: Option<&str>, _help: Option<&str>) -> Result<String> {
        match self.next(label)? {
            // An empty answer takes the default, as in a terminal.
            Answer::Text(t) if t.is_empty() => Ok(default.unwrap_or_default().to_string()),
            Answer::Text(t) => Ok(t),
            other => Err(anyhow!("`{label}` wants text, got {other:?}")),
        }
    }

    fn select(
        &mut self,
        label: &str,
        options: &[String],
        _default: Option<usize>,
        _help: Option<&str>,
    ) -> Result<usize> {
        match self.next(label)? {
            Answer::Select(i) if i < options.len() => Ok(i),
            other => Err(anyhow!("`{label}` wants a choice, got {other:?}")),
        }
    }

    fn multi_select(
        &mut self,
        label: &str,
        options: &[String],
        _defaults: &[usize],
        _help: Option<&str>,
    ) -> Result<Vec<usize>> {
        match self.next(label)? {
            Answer::Multi(v) if v.iter().all(|i| *i < options.len()) => Ok(v),
            other => Err(anyhow!("`{label}` wants choices, got {other:?}")),
        }
    }

    fn confirm(&mut self, label: &str, _default: bool) -> Result<bool> {
        match self.next(label)? {
            Answer::Confirm(b) => Ok(b),
            other => Err(anyhow!("`{label}` wants yes/no, got {other:?}")),
        }
    }

    fn secret(&mut self, label: &str) -> Result<String> {
        match self.next(label)? {
            Answer::Secret(s) => Ok(s),
            other => Err(anyhow!("`{label}` wants a secret, got {other:?}")),
        }
    }
}

/// Checks a value; `Err` is shown to the person (or fails the command for a flag value).
pub type Validator<T> = fn(&T) -> std::result::Result<(), String>;

/// A question and the flag that answers it.
pub struct Ask<T> {
    label: String,
    flag: String,
    value: Option<T>,
    default: Option<T>,
    validate: Option<Validator<T>>,
    help: Option<String>,
}

impl<T> Ask<T> {
    /// `label` is the question, `flag` the long flag that answers it (`--workflow`).
    pub fn new(label: impl Into<String>, flag: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            flag: flag.into(),
            value: None,
            default: None,
            validate: None,
            help: None,
        }
    }

    /// The value given on the command line, if any.
    pub fn value(mut self, value: Option<T>) -> Self {
        self.value = value;
        self
    }

    pub fn default(mut self, value: T) -> Self {
        self.default = Some(value);
        self
    }

    pub fn default_opt(mut self, value: Option<T>) -> Self {
        self.default = value;
        self
    }

    pub fn validate(mut self, check: Validator<T>) -> Self {
        self.validate = Some(check);
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    pub fn flag(&self) -> &str {
        &self.flag
    }
}

/// One option of a select: the value, what is shown, and an optional muted hint.
#[derive(Debug, Clone)]
pub struct Choice<T> {
    pub value: T,
    pub label: String,
    pub hint: Option<String>,
}

impl<T> Choice<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            hint: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    fn shown(&self) -> String {
        match &self.hint {
            Some(h) => format!("{} — {h}", self.label),
            None => self.label.clone(),
        }
    }
}

/// How many times a person may give an invalid answer before the command fails.
const MAX_TRIES: usize = 5;

/// Reads an environment variable.
type EnvFn<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// A wizard: questions with flags, `--yes` for scripts, and the equivalent command at the end.
pub struct Wizard<'a> {
    ui: &'a Ui,
    yes: bool,
    interactive: bool,
    term: Box<dyn Term + 'a>,
    env: EnvFn<'a>,
    flags: Vec<(String, Option<String>)>,
    asked: bool,
}

/// Quotes `value` for a POSIX shell when needed.
pub fn shell_quote(value: &str) -> String {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:@,=+%".contains(c));
    if safe {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn invalid(flag: &str, message: String) -> anyhow::Error {
    UsageError::new("invalid_flag", format!("{flag}: {message}"))
        .field(flag)
        .into()
}

impl<'a> Wizard<'a> {
    /// A wizard on the real terminal; questions are asked only when stdin and stdout are terminals (and not
    /// `--json`).
    pub fn new(ui: &'a Ui, yes: bool) -> Self {
        let mode = *ui.mode();
        Wizard::with_term(
            ui,
            yes,
            mode.interactive && !mode.json,
            Box::new(InquireTerm::for_mode(&mode)),
        )
    }

    pub fn with_term(ui: &'a Ui, yes: bool, interactive: bool, term: Box<dyn Term + 'a>) -> Self {
        Self {
            ui,
            yes,
            interactive,
            term,
            env: Box::new(|name| std::env::var(name).ok()),
            flags: Vec::new(),
            asked: false,
        }
    }

    /// Reads environment variables through `env` (tests).
    pub fn with_env(mut self, env: impl Fn(&str) -> Option<String> + 'a) -> Self {
        self.env = Box::new(env);
        self
    }

    pub fn yes(&self) -> bool {
        self.yes
    }

    pub fn interactive(&self) -> bool {
        self.interactive
    }

    /// Adds `flag value` (or a bare `flag` with `None`) to the equivalent command.
    pub fn record(&mut self, flag: impl Into<String>, value: Option<String>) {
        let flag = flag.into();
        self.flags.retain(|(f, _)| *f != flag);
        self.flags.push((flag, value));
    }

    /// Decides where an answer comes from when no flag gave it: the default with `--yes`, an error without a
    /// terminal, else `None` (ask).
    fn without_flag<T: Clone>(&mut self, q: &Ask<T>) -> Result<Option<T>> {
        if self.yes {
            return match &q.default {
                Some(d) => {
                    self.asked = true;
                    Ok(Some(d.clone()))
                }
                None => Err(UsageError::missing_flag(&q.flag, false).into()),
            };
        }
        if !self.interactive {
            return Err(UsageError::missing_flag(&q.flag, true).into());
        }
        self.asked = true;
        Ok(None)
    }

    fn report_invalid(&self, message: &str) {
        self.ui
            .eline(&format!("{} {message}", self.ui.paint(Token::Coral, "✕")));
    }

    /// A text answer.
    pub fn text(&mut self, q: Ask<String>) -> Result<String> {
        if let Some(v) = q.value.clone() {
            if let Some(check) = q.validate {
                check(&v).map_err(|m| invalid(&q.flag, m))?;
            }
            self.record(q.flag.clone(), Some(v.clone()));
            return Ok(v);
        }
        if let Some(v) = self.without_flag(&q)? {
            self.record(q.flag.clone(), Some(v.clone()));
            return Ok(v);
        }
        for _ in 0..MAX_TRIES {
            let v = self
                .term
                .text(&q.label, q.default.as_deref(), q.help.as_deref())?;
            match q.validate.map(|check| check(&v)).unwrap_or(Ok(())) {
                Ok(()) => {
                    self.record(q.flag.clone(), Some(v.clone()));
                    return Ok(v);
                }
                Err(m) => self.report_invalid(&m),
            }
        }
        Err(invalid(&q.flag, "too many invalid answers".into()))
    }

    /// One of `options`. A flag value does not have to be one of them (free values such as a template slug), but
    /// must pass the validator.
    pub fn select<T: Clone + PartialEq + Display>(&mut self, q: Ask<T>, options: &[Choice<T>]) -> Result<T> {
        if let Some(v) = q.value.clone() {
            if let Some(check) = q.validate {
                check(&v).map_err(|m| invalid(&q.flag, m))?;
            }
            self.record(q.flag.clone(), Some(v.to_string()));
            return Ok(v);
        }
        if let Some(v) = self.without_flag(&q)? {
            self.record(q.flag.clone(), Some(v.to_string()));
            return Ok(v);
        }
        if options.is_empty() {
            return Err(anyhow!("nothing to choose from for {}", q.flag));
        }
        let shown: Vec<String> = options.iter().map(Choice::shown).collect();
        let default = q
            .default
            .as_ref()
            .and_then(|d| options.iter().position(|o| &o.value == d));
        for _ in 0..MAX_TRIES {
            let i = self.term.select(&q.label, &shown, default, q.help.as_deref())?;
            let v = options[i].value.clone();
            match q.validate.map(|check| check(&v)).unwrap_or(Ok(())) {
                Ok(()) => {
                    self.record(q.flag.clone(), Some(v.to_string()));
                    return Ok(v);
                }
                Err(m) => self.report_invalid(&m),
            }
        }
        Err(invalid(&q.flag, "too many invalid answers".into()))
    }

    /// Several of `options` (`--skills a,b`).
    pub fn multi_select<T: Clone + PartialEq + Display>(
        &mut self,
        q: Ask<Vec<T>>,
        options: &[Choice<T>],
    ) -> Result<Vec<T>> {
        let joined = |v: &[T]| v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
        if let Some(v) = q.value.clone() {
            if let Some(check) = q.validate {
                check(&v).map_err(|m| invalid(&q.flag, m))?;
            }
            self.record(q.flag.clone(), Some(joined(&v)));
            return Ok(v);
        }
        if let Some(v) = self.without_flag(&q)? {
            self.record(q.flag.clone(), Some(joined(&v)));
            return Ok(v);
        }
        let shown: Vec<String> = options.iter().map(Choice::shown).collect();
        let defaults: Vec<usize> = q
            .default
            .as_ref()
            .map(|d| {
                options
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| d.contains(&o.value))
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        for _ in 0..MAX_TRIES {
            let picked = self
                .term
                .multi_select(&q.label, &shown, &defaults, q.help.as_deref())?;
            let v: Vec<T> = picked.iter().map(|i| options[*i].value.clone()).collect();
            match q.validate.map(|check| check(&v)).unwrap_or(Ok(())) {
                Ok(()) => {
                    self.record(q.flag.clone(), Some(joined(&v)));
                    return Ok(v);
                }
                Err(m) => self.report_invalid(&m),
            }
        }
        Err(invalid(&q.flag, "too many invalid answers".into()))
    }

    /// Yes or no before doing something. `--yes` → yes; no terminal and no `--yes` → `confirmation_required`.
    pub fn confirm(&mut self, label: &str) -> Result<bool> {
        if self.yes {
            return Ok(true);
        }
        if !self.interactive {
            return Err(UsageError::confirmation_required(label.trim_end_matches('?')).into());
        }
        self.term.confirm(label, false)
    }

    /// A secret, read from the environment variable named by `flag` (`--secret-env VAR`); the secret itself is never
    /// a flag value. Without the flag it is asked with hidden input in a terminal.
    pub fn secret_from_env(&mut self, label: &str, flag: &str, var: Option<&str>) -> Result<String> {
        if let Some(var) = var {
            let value = (self.env)(var).filter(|v| !v.is_empty()).ok_or_else(|| {
                anyhow::Error::from(
                    UsageError::new(
                        "missing_flag",
                        format!("the environment variable {var} is empty or unset"),
                    )
                    .field(flag)
                    .fix(format!(
                        "export {var}=… before running, or omit {flag} to type it"
                    )),
                )
            })?;
            self.record(flag, Some(var.to_string()));
            return Ok(value);
        }
        if self.yes || !self.interactive {
            return Err(UsageError::missing_flag(flag, !self.yes).into());
        }
        self.asked = true;
        let value = self.term.secret(label)?;
        // The equivalent command names a variable to hold it, never the secret.
        self.record(flag, Some("<VAR>".into()));
        Ok(value)
    }

    /// `ttm <command> --flag value … --yes`.
    pub fn equivalent(&self, command: &str) -> String {
        let mut out = format!("ttm {command}");
        for (flag, value) in &self.flags {
            out.push(' ');
            out.push_str(flag);
            if let Some(v) = value {
                out.push(' ');
                out.push_str(&shell_quote(v));
            }
        }
        out.push_str(" --yes");
        out
    }

    /// Prints `Equivalent: <command>` on stderr when at least one answer did not come from a flag.
    pub fn print_equivalent(&self, command: &str) {
        if !self.asked {
            return;
        }
        let line = self.equivalent(command);
        self.ui.eline(&format!(
            "{} {}",
            self.ui.paint(Token::Muted, "Equivalent:"),
            self.ui.paint(Token::Blue, &line)
        ));
    }
}
