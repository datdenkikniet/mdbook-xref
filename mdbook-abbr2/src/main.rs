use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result};
use mdbook_preprocessor::{
    PreprocessorContext,
    book::{Book, BookItem, Chapter},
};
use pulldown_cmark::{Event, LinkType, Tag};
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

    do_rewrite(&abbreviations, &mut used_abbreviations, &mut book.items)?;

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

fn do_rewrite(
    abbrs: &HashMap<String, Abbreviation>,
    used: &mut HashSet<String>,
    items: &mut [BookItem],
) -> Result<()> {
    let chapters = items.iter_mut().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    let mut encountered = HashSet::<String>::new();

    for chapter in chapters {
        let content = &chapter.content;
        let parser = pulldown_cmark::Parser::new(content).into_offset_iter();

        let mut replacements = Vec::new();

        for (event, range) in parser {
            let Event::Start(Tag::Link {
                link_type: LinkType::Autolink,
                dest_url,
                ..
            }) = event
            else {
                continue;
            };

            let Some(abbr) = dest_url.strip_prefix("abbr:") else {
                continue;
            };

            let (abbr, _form) = abbr
                .rsplit_once(':')
                .map(|(a, b)| (a, Some(b)))
                .unwrap_or((abbr, None));

            let Some(abbr) = abbrs.get(abbr) else {
                anyhow::bail!("Unknown abbreviation '{abbr}' used ");
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
            let link = if encountered.contains(abbr) {
                format!(r#"[{abbr}](ref:abbr-{abbr} "{hover}")"#)
            } else {
                format!(r#"[{exp} ({abbr})](ref:abbr-{abbr})"#)
            };

            encountered.insert(abbr.clone());

            let replacement = format!(r#"<span class="abbr">{link}</span>"#);

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

        do_rewrite(abbrs, used, &mut chapter.sub_items)?;

        // Ensure the first of each abbr is expanded in each chapter
        encountered.clear();
    }
    Ok(())
}
