use std::{borrow::Cow, collections::HashMap};

use fluent_bundle::FluentValue;
use fluent_templates::{Loader, StaticLoader};
use tracing::{debug, warn};

use crate::{Context, Data};

#[macro_export]
macro_rules! t {
    ($ctx:expr, $text_id:expr) => {
        $crate::translation::lookup_ctx($ctx, &*$crate::LOCALES, $text_id, None)
    };

    ($ctx:expr, $text_id:expr, $($key:expr => $value:expr),+ $(,)?) => {{
        let mut args = std::collections::HashMap::new();
        $(
            args.insert(
                std::borrow::Cow::Borrowed($key),
                fluent_bundle::FluentValue::from($value)
            );
        )+
        $crate::translation::lookup_ctx($ctx, &*$crate::LOCALES, $text_id, Some(&args))
    }};
}

pub fn lookup_ctx(
    ctx: &Context<'_>,
    locales: &StaticLoader,
    text_id: &str,
    args: Option<&HashMap<Cow<'static, str>, FluentValue>>,
) -> String {
    let lang = ctx
        .locale()
        .and_then(|l| l.parse().ok())
        .unwrap_or_else(|| locales.fallback().clone());

    locales
        .try_lookup_complete(&lang, text_id, args)
        .unwrap_or_else(|| text_id.to_owned())
}

pub fn apply_translations(
    locales: &StaticLoader,
    commands: &mut [poise::Command<Data, anyhow::Error>],
) {
    debug!("Applying translations for {} command(s)", commands.len());
    apply_translations_recursive(locales, commands, None);
}

// TODO: rewrite
fn apply_translations_recursive(
    locales: &StaticLoader,
    commands: &mut [poise::Command<Data, anyhow::Error>],
    parent_key: Option<&str>,
) {
    let fallback_lang = locales.fallback();
    let other_langs: Vec<_> = locales
        .locales()
        .filter(|&lang| lang != fallback_lang)
        .cloned()
        .collect();

    match parent_key {
        Some(parent) => debug!("Translating subcommands for '{}'...", parent),
        None => debug!(
            "Language setup: fallback is '{}', translating to: {:?}",
            fallback_lang, other_langs
        ),
    }

    for command in commands {
        let base_key = match parent_key {
            Some(parent) => format!("{}-{}", parent, command.name),
            None => command.name.clone(),
        };

        let Some(translated_name) = locales.try_lookup(fallback_lang, &base_key) else {
            warn!(
                "Skipping command '{}': missing base translation for key '{}'",
                command.name, base_key
            );
            continue;
        };

        debug!("Command '{}':", command.name);

        let description_key = format!("{}.description", base_key);

        if let Some(x) = locales.try_lookup(fallback_lang, &description_key) {
            command.description = Some(x);
        }

        for lang in &other_langs {
            if let Some(x) = locales.try_lookup(lang, &base_key) {
                debug!("Translated name to '{}'", lang);
                command.name_localizations.insert(lang.to_string(), x);
            }

            if let Some(x) = locales.try_lookup(lang, &description_key) {
                debug!("Translated description to '{}'", lang);
                command
                    .description_localizations
                    .insert(lang.to_string(), x);
            }
        }

        for parameter in &mut command.parameters {
            let key = format!("{}.{}", base_key, parameter.name);
            let description_key = format!("{key}-description");

            debug!("Parameter '{}':", parameter.name);

            if let Some(x) = locales.try_lookup(fallback_lang, &description_key) {
                parameter.description = Some(x);
            }

            for lang in &other_langs {
                if let Some(x) = locales.try_lookup(lang, &key) {
                    debug!("Translated name to '{}'", lang);
                    parameter.name_localizations.insert(lang.to_string(), x);
                }

                if let Some(x) = locales.try_lookup(lang, &description_key) {
                    debug!("Translated description to '{}'", lang);
                    parameter
                        .description_localizations
                        .insert(lang.to_string(), x);
                }
            }

            for choice in &mut parameter.choices {
                let choice_key = format!("{key}-{}", choice.name);

                for lang in &other_langs {
                    if let Some(x) = locales.try_lookup(lang, &choice_key) {
                        debug!("Translated choice '{}' to '{}'", choice.name, lang);
                        choice.localizations.insert(lang.to_string(), x);
                    }
                }

                if let Some(x) = locales.try_lookup(fallback_lang, &choice_key) {
                    choice.name = x;
                }
            }

            if let Some(x) = locales.try_lookup(fallback_lang, &key) {
                parameter.name = x;
            }
        }

        if !command.subcommands.is_empty() {
            debug!(
                "Found {} subcommand(s) for '{}'",
                command.subcommands.len(),
                command.name
            );
        }

        apply_translations_recursive(locales, &mut command.subcommands, Some(&base_key));

        command.name = translated_name;
    }
}
