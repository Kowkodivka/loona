use std::{borrow::Cow, collections::HashMap};

use fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, StaticLoader};
use tracing::{debug, info, warn};

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
    loader: &StaticLoader,
    commands: &mut [poise::Command<Data, anyhow::Error>],
) {
    let fallback_lang = loader.fallback();
    let other_langs: Vec<_> = loader.locales().filter(|&l| l != fallback_lang).collect();

    if other_langs.is_empty() {
        warn!("No locales besides fallback '{fallback_lang}' - localizations will be empty");
    }

    info!(
        "Applying translations: {} command(s), fallback '{fallback_lang}', {} other locale(s)",
        commands.len(),
        other_langs.len()
    );

    apply_translations_recursive(loader, fallback_lang, &other_langs, commands, None);
}

fn localize(
    loader: &StaticLoader,
    fallback_lang: &LanguageIdentifier,
    other_langs: &[&LanguageIdentifier],
    key: &str,
    localizations: &mut HashMap<String, String>,
) -> Option<String> {
    for &lang in other_langs {
        match loader.try_lookup(lang, key) {
            Some(x) => {
                localizations.insert(lang.to_string(), x);
            }
            None => debug!("no '{lang}' translation for '{key}'"),
        }
    }

    loader.try_lookup(fallback_lang, key)
}

fn apply_translations_recursive(
    loader: &StaticLoader,
    fallback_lang: &LanguageIdentifier,
    other_langs: &[&LanguageIdentifier],
    commands: &mut [poise::Command<Data, anyhow::Error>],
    parent_key: Option<&str>,
) {
    for command in commands {
        let base_key = match parent_key {
            Some(parent) => format!("{parent}-{}", command.name),
            None => command.name.clone(),
        };

        let Some(name) = localize(
            loader,
            fallback_lang,
            other_langs,
            &base_key,
            &mut command.name_localizations,
        ) else {
            warn!("No fallback translation for command '{base_key}', skipping");
            continue;
        };

        let description_key = format!("{base_key}.description");
        command.description = localize(
            loader,
            fallback_lang,
            other_langs,
            &description_key,
            &mut command.description_localizations,
        );

        if command.description.is_none() {
            warn!("Command '{base_key}' has a name but no description ('{description_key}')");
        }

        for parameter in &mut command.parameters {
            let key = format!("{base_key}.{}", parameter.name);
            let description_key = format!("{key}-description");

            if let Some(x) = localize(
                loader,
                fallback_lang,
                other_langs,
                &key,
                &mut parameter.name_localizations,
            ) {
                parameter.name = x;
            } else {
                warn!("No fallback translation for parameter '{key}'");
            }

            parameter.description = localize(
                loader,
                fallback_lang,
                other_langs,
                &description_key,
                &mut parameter.description_localizations,
            );

            if parameter.description.is_none() {
                warn!("Parameter '{key}' has a name but no description ('{description_key}')");
            }

            for choice in &mut parameter.choices {
                let choice_key = format!("{key}-{}", choice.name);

                if let Some(x) = localize(
                    loader,
                    fallback_lang,
                    other_langs,
                    &choice_key,
                    &mut choice.localizations,
                ) {
                    choice.name = x;
                } else {
                    warn!("No fallback translation for choice '{choice_key}'");
                }
            }
        }

        if !command.subcommands.is_empty() {
            apply_translations_recursive(
                loader,
                fallback_lang,
                other_langs,
                &mut command.subcommands,
                Some(&base_key),
            );
        }

        command.name = name;

        debug!("translated command '{base_key}'");
    }
}
