//! Utilities for configuring the use of [`minijinja`]
use minijinja::Environment;
use minijinja_autoreload::AutoReloader;

use crate::Configuration;

/// type-alias shorthand for the underlying environment type.
pub type Jinja = AutoReloader;

/// given a [`Config`] produce an [`minijinja::Environment`] to load html templates from
pub fn make_jinja_env(config: &Configuration) -> Jinja {
    let path = config.jinja_template_dir();
    AutoReloader::new(move |notif| {
        tracing::warn!("JINJA TEMPALTE RELOAD");
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(&path));
        notif.watch_path(&path, true);
        Ok(env)
    })
}
