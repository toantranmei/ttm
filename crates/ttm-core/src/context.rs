//! Everything a command needs to know before talking to a server: paths, config, profile name and host.

use crate::profile::{resolve_host, resolve_profile_name};
use crate::{Config, CredentialStore, Paths, Profile, Result};

/// Values from the command line (`--host`, `--profile`). Environment variables are read by [`Context::load`].
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub host: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub paths: Paths,
    pub config: Config,
    pub profile_name: String,
    pub host: String,
    pub credentials: CredentialStore,
}

impl Context {
    /// Loads from the real environment (`TTM_HOST`, `TTM_PROFILE`, `TTM_TOKEN`, `TTM_CONFIG_DIR`, …).
    pub fn load(overrides: &Overrides) -> Result<Self> {
        let paths = Paths::detect()?;
        let env = |name: &str| std::env::var(name).ok();
        let credentials = CredentialStore::from_env(&paths);
        Self::resolve(paths, overrides, env("TTM_HOST"), env("TTM_PROFILE"), credentials)
    }

    /// Same as [`Context::load`] with every input explicit (tests).
    pub fn resolve(
        paths: Paths,
        overrides: &Overrides,
        env_host: Option<String>,
        env_profile: Option<String>,
        credentials: CredentialStore,
    ) -> Result<Self> {
        let config = Config::load(&paths.config_file())?;
        let profile_name = resolve_profile_name(
            overrides.profile.as_deref(),
            env_profile.as_deref(),
            config.default_profile.as_deref(),
        );
        let profile_host = config.profile(&profile_name).and_then(|p| p.host.clone());
        let host = resolve_host(
            overrides.host.as_deref(),
            env_host.as_deref(),
            profile_host.as_deref(),
        )?;
        Ok(Self {
            paths,
            config,
            profile_name,
            host,
            credentials,
        })
    }

    pub fn profile(&self) -> Profile {
        self.config
            .profile(&self.profile_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Current workspace of the profile, if one was chosen.
    pub fn workspace(&self) -> Option<String> {
        self.profile().workspace
    }

    /// Applies `edit` to the profile and saves `config.toml`.
    pub fn update_profile(&mut self, edit: impl FnOnce(&mut Profile)) -> Result<()> {
        edit(self.config.profile_mut(&self.profile_name));
        self.config.save(&self.paths.config_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &std::path::Path, overrides: Overrides, env_host: Option<&str>) -> Context {
        let paths = Paths::under(dir);
        let creds = CredentialStore::new(&paths, false, None);
        Context::resolve(paths, &overrides, env_host.map(String::from), None, creds).unwrap()
    }

    #[test]
    fn profile_host_is_used_and_flag_wins() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let mut config = Config {
            default_profile: Some("dev".into()),
            ..Config::default()
        };
        config.profile_mut("dev").host = Some("http://localhost:3000".into());
        config.save(&paths.config_file()).unwrap();

        let c = ctx(dir.path(), Overrides::default(), None);
        assert_eq!(
            (c.profile_name.as_str(), c.host.as_str()),
            ("dev", "http://localhost:3000")
        );
        let c = ctx(dir.path(), Overrides::default(), Some("https://env.com"));
        assert_eq!(c.host, "https://env.com");
        let c = ctx(
            dir.path(),
            Overrides {
                host: Some("https://flag.com".into()),
                profile: None,
            },
            Some("https://env.com"),
        );
        assert_eq!(c.host, "https://flag.com");
    }

    #[test]
    fn update_profile_persists() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = ctx(
            dir.path(),
            Overrides {
                host: None,
                profile: Some("work".into()),
            },
            None,
        );
        c.update_profile(|p| p.workspace = Some("ws-9".into())).unwrap();
        let again = ctx(
            dir.path(),
            Overrides {
                host: None,
                profile: Some("work".into()),
            },
            None,
        );
        assert_eq!(again.workspace().as_deref(), Some("ws-9"));
    }
}
