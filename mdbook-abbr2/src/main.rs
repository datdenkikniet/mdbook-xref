use std::{
    collections::{HashMap, HashSet},
    io::Write,
    ops::Deref,
    path::PathBuf,
};

use serde::Deserialize;

use anyhow::{Context, Result};
use mdbook_preprocessor::{
    PreprocessorContext,
    book::{Book, BookItem, Chapter},
};
use pulldown_cmark::{CowStr, Event, LinkType, OffsetIter, Tag, TagEnd};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let command = args.get(0);

    match command.as_ref().map(|v| v.as_str()) {
        Some("supports") => {
            let backend = args
                .get(1)
                .context("missing 2nd argument specifying backend")?;

            return if backend == "html" {
                Ok(())
            } else {
                Err(anyhow::anyhow!("{backend} backend is not supported."))
            };
        }
        Some(_) => return Err(anyhow::anyhow!("Unknown command")),
        _ => {
            let book = book()?;
            std::io::stdout().write_all(book.as_bytes())?;
            Ok(())
        }
    }
}

fn book() -> Result<String> {
    let (ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;

    rewrite_book(&ctx, &mut book)?;

    Ok(serde_json::to_string(&book)?)
}

#[derive(serde::Deserialize)]
struct Abbreviation {
    pub abbreviation: String,
    pub expanded: String,
    pub hover: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ValidationMode {
    Quiet,
    #[default]
    Warn,
    Error,
}

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    Warning,
    Error,
}

#[derive(Default)]
struct Diagnostics(Vec<(Severity, String)>);

impl Diagnostics {
    fn push(&mut self, severity: Severity, message: String) {
        self.0.push((severity, message));
    }

    /// Print every diagnostic, failing if any of them is an error.
    fn report(&self) -> Result<()> {
        for (severity, message) in &self.0 {
            match severity {
                Severity::Warning => eprintln!("Warning: {message}"),
                Severity::Error => eprintln!("Error: {message}"),
            }
        }

        let errors = self.0.iter().filter(|(s, _)| *s == Severity::Error).count();

        if errors > 0 {
            let plural = if errors == 1 { "" } else { "s" };
            anyhow::bail!("aborting due to {errors} previous error{plural}");
        }

        Ok(())
    }
}

struct Configuration {
    pub auto_expand: bool,
    pub validation: Option<Severity>,
    pub excluded_chapters: HashSet<PathBuf>,
}

fn rewrite_book(ctx: &PreprocessorContext, book: &mut Book) -> Result<()> {
    let abbr_path: PathBuf = ctx
        .config
        .get("preprocessor.abbr2.path")?
        .context("No abbreviations path configured.")?;

    let abbr_path = ctx.root.join(abbr_path);
    let data = std::fs::read(&abbr_path)
        .with_context(|| format!("Failed to read abbreviations file {}", abbr_path.display()))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(data.as_slice());

    let mut abbreviations = HashMap::new();

    for abbreviation in reader.records() {
        let abbreviation = abbreviation.context("Failed to deserialize a CSV record")?;

        let mut fields = abbreviation.iter();

        let Some(abbreviation) = fields.next() else {
            anyhow::bail!("Expected 2 or 3 columns per CSV row, got none");
        };

        let Some(expanded) = fields.next() else {
            anyhow::bail!("No expanded form defined for '{abbreviation}'");
        };

        let hover = fields.next().map(|v| v.to_string());

        if abbreviations
            .insert(
                abbreviation.to_string(),
                Abbreviation {
                    abbreviation: abbreviation.to_string(),
                    expanded: expanded.to_string(),
                    hover,
                },
            )
            .is_some()
        {
            anyhow::bail!("Abbreviation '{abbreviation}' defined more than once");
        }
    }

    let mut used_abbreviations = HashSet::new();

    let validation_mode: ValidationMode = ctx
        .config
        .get("preprocessor.abbr2.validate")?
        .unwrap_or_default();

    let config = Configuration {
        auto_expand: ctx
            .config
            .get("preprocessor.abbr2.auto-expand")?
            .unwrap_or(true),
        validation: match validation_mode {
            ValidationMode::Quiet => None,
            ValidationMode::Warn => Some(Severity::Warning),
            ValidationMode::Error => Some(Severity::Error),
        },
        excluded_chapters: ctx
            .config
            .get("preprocessor.abbr2.exclude-chapters")?
            .unwrap_or_default(),
    };

    let mut diagnostics = Diagnostics::default();

    do_rewrite(
        &abbreviations,
        &mut used_abbreviations,
        &mut book.items,
        &config,
        &mut diagnostics,
    );

    diagnostics.report()?;

    if !used_abbreviations.is_empty() {
        let separator = ctx
            .config
            .get("preprocessor.abbr2.separator")?
            .unwrap_or(true);

        if separator {
            book.items.push(BookItem::Separator);
        }

        let chapter = make_abbr_chapter(&abbreviations, &mut used_abbreviations);

        book.items.push(BookItem::Chapter(chapter));
    }

    Ok(())
}

fn make_abbr_chapter(abbrs: &HashMap<String, Abbreviation>, used: &HashSet<String>) -> Chapter {
    let mut page = String::new();

    let mut used = used.into_iter().collect::<Vec<_>>();
    used.sort();

    page.push_str(
        r#"| Abbreviation | Definition |
| :----------- | :--------- |"#,
    );
    page.push('\n');

    for abbr in used {
        let expanded = &abbrs.get(abbr).unwrap().expanded;
        let id = format!("abbr-{abbr}");
        let entry = format!(r#"| [**{abbr}**](label:{id} "{abbr}") | {expanded} |"#);

        page.push_str(&entry);
        page.push('\n');
    }

    let chapter = Chapter {
        name: "Abbreviations".into(),
        content: page,
        number: None,
        sub_items: Vec::new(),
        path: Some("abbreviations.md".into()),
        source_path: None,
        parent_names: Default::default(),
    };

    chapter
}

/// Skip through event types that we do not care about
fn skip_event(parser: &mut OffsetIter<'_>, end_type: TagEnd) {
    while parser
        .next()
        .is_some_and(|(e, _)| e != Event::End(end_type))
    {}
}

fn print_chapter_info(chapter: &Chapter) -> String {
    match chapter.path.as_ref() {
        Some(path) => path.display().to_string(),
        None => chapter.name.clone(),
    }
}

fn do_rewrite(
    abbrs: &HashMap<String, Abbreviation>,
    used: &mut HashSet<String>,
    items: &mut [BookItem],
    config: &Configuration,
    diagnostics: &mut Diagnostics,
) {
    let chapters = items.iter_mut().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    let mut encountered = HashSet::<String>::new();

    for chapter in chapters {
        let content = &chapter.content;

        // Check if chapter is marked for skipping
        if chapter
            .path
            .as_ref()
            .is_some_and(|p| config.excluded_chapters.iter().any(|e| p.starts_with(e)))
        {
            eprintln!("Skipping chapter {}", print_chapter_info(chapter));
            continue;
        }

        let mut parser = pulldown_cmark::Parser::new(content).into_offset_iter();

        let mut replacements = Vec::new();
        let mut dest_url: CowStr<'_>;

        //for (event, range) in parser {
        while let Some((event, range)) = parser.next() {
            match &event {
                Event::Start(Tag::Link {
                    link_type: LinkType::Autolink,
                    dest_url: url,
                    ..
                }) => {
                    dest_url = url.clone();
                }
                Event::Start(Tag::Link { .. }) => {
                    skip_event(&mut parser, TagEnd::Link);
                    continue;
                }
                // Consume code block to skip validation
                Event::Start(Tag::CodeBlock(_)) => {
                    skip_event(&mut parser, TagEnd::CodeBlock);
                    continue;
                }
                Event::Text(text) => {
                    let Some(severity) = config.validation else {
                        continue;
                    };

                    for err_word in check_text(abbrs, text.deref()) {
                        diagnostics.push(
                            severity,
                            format!(
                                "{} is recognized as an abbreviation, but not marked as such. Chapter {}, somewhere in {}",
                                err_word,
                                print_chapter_info(chapter),
                                &content[range.clone()],
                            ),
                        );
                    }
                    continue;
                }
                _ => continue,
            };

            let Some((mark_abbr, abbr)) = parse_abbreviation(&dest_url) else {
                continue;
            };

            let replacement = if mark_abbr {
                let Some(replacement) = create_abbr_replacement(
                    abbr,
                    abbrs,
                    used,
                    &mut encountered,
                    config,
                    diagnostics,
                ) else {
                    continue;
                };
                replacement
            } else {
                abbr.to_string()
            };

            replacements.push((range, replacement));
        }

        let mut output = String::new();
        let mut last_copied = 0;
        for (range, replacement) in replacements {
            output.push_str(&content[last_copied..range.start]);
            last_copied = range.end;

            output.push_str(&replacement);
        }

        output.push_str(&content[last_copied..]);

        chapter.content = output;

        do_rewrite(abbrs, used, &mut chapter.sub_items, config, diagnostics);

        // Ensure the first of each abbr is expanded in each chapter
        encountered.clear();
    }
}

fn create_abbr_replacement(
    abbr: &str,
    abbrs: &HashMap<String, Abbreviation>,
    used: &mut HashSet<String>,
    encountered: &mut HashSet<String>,
    config: &Configuration,
    diagnostics: &mut Diagnostics,
) -> Option<String> {
    let (abbr, _form) = abbr
        .rsplit_once(':')
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((abbr, None));

    let Some(abbr) = abbrs.get(abbr) else {
        diagnostics.push(
            Severity::Error,
            format!("Unknown abbreviation '{abbr}' used"),
        );
        return None;
    };

    used.insert(abbr.abbreviation.clone());

    let hover = abbr
        .hover
        .as_ref()
        .unwrap_or_else(|| &abbr.expanded)
        .replace(r#"""#, r#"\""#);

    let exp: &String = abbr.hover.as_ref().unwrap_or(&abbr.expanded);
    let abbr = &abbr.abbreviation;

    // first time expansion of abbreviation in chapter
    let link = if !config.auto_expand || encountered.contains(abbr) {
        format!(r#"[{abbr}](ref:abbr-{abbr} "{hover}")"#)
    } else {
        format!(r#"[{exp} ({abbr})](ref:abbr-{abbr})"#)
    };

    encountered.insert(abbr.clone());

    Some(format!(r#"<span class="abbr">{link}</span>"#))
}

fn parse_abbreviation<'a>(dest_url: &'a CowStr<'a>) -> Option<(bool, &'a str)> {
    if let Some(rest) = dest_url.strip_prefix("abbr:") {
        Some((true, rest))
    } else if let Some(rest) = dest_url.strip_prefix("noabbr:") {
        Some((false, rest))
    } else {
        None
    }
}
fn check_text<'a>(
    abbrs: &'a HashMap<String, Abbreviation>,
    text: &'a str,
) -> impl Iterator<Item = &'a str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|c: char| c.is_ascii_punctuation()))
        .filter(|word| abbrs.contains_key(*word))
}
