use std::io::Write;
use std::ops::Range;
use std::{collections::HashMap, fmt::Write as _};

use anyhow::{Context, Result};
use mdbook_preprocessor::book::BookItem;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let command = args.get(0);

    let output = match command.as_ref().map(|v| v.as_str()) {
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
        _ => book()?,
    };

    std::io::stdout().write_all(output.as_bytes())?;

    Ok(())
}

struct Figure<'a> {
    input_range: Range<usize>,
    description: Option<&'a str>,
    replacement_text: &'a str,
    label: String,
    ty: String,
    counter: usize,
}

fn book() -> Result<String> {
    let (_ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;
    number_figures(&mut book.items, &mut Default::default())?;
    Ok(serde_json::to_string(&book)?)
}

fn number_figures(items: &mut [BookItem], counters: &mut HashMap<String, usize>) -> Result<()> {
    let chapters = items.iter_mut().filter_map(|i| {
        if let BookItem::Chapter(c) = i {
            Some(c)
        } else {
            None
        }
    });

    for chapter in chapters {
        let content = &chapter.content;
        let mut parser = Parser::new(&content).into_offset_iter();

        let mut rewrites = Vec::new();
        while let Some((next, range)) = parser.next() {
            let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = next else {
                continue;
            };

            let data: Vec<_> = info.splitn(3, ' ').collect();

            let Some((maybe_figure, data)) = data.split_first() else {
                continue;
            };

            if maybe_figure != &"figure" {
                continue;
            }

            let Some((label, data)) = data.split_first() else {
                anyhow::bail!(
                    "Encountered figure without label in chapter {name}!",
                    name = chapter.name
                );
            };

            let ty = data.get(0);

            let (mut current, mut current_range) = parser.next().unwrap();
            let start = current_range.start;
            let mut end = current_range.end;
            loop {
                if current == Event::End(TagEnd::CodeBlock) {
                    break;
                } else {
                    end = current_range.end;
                }

                let (next, range) = parser.next().unwrap();
                current = next;
                current_range = range;
            }

            let inner_text = if (start..end) != current_range {
                &content[start..end]
            } else {
                ""
            };
            let description = inner_text
                .lines()
                .next()
                .map(|v| v.trim())
                .and_then(|v| (!v.is_empty()).then_some(v));
            let skip = description.map(|v| v.len() + 1).unwrap_or(0);

            let replacement_text = &inner_text[skip..];

            let ty = if let Some(ty) = ty {
                ty.to_string()
            } else {
                let mut parser = Parser::new_ext(replacement_text, Options::ENABLE_TABLES);
                let first = parser.next();

                if let Some(Event::Start(Tag::Table(_))) = first {
                    "Table".to_string()
                } else {
                    "Figure".to_string()
                }
            };

            let counter = counters.entry(ty.to_string()).or_default();

            let figure = Figure {
                input_range: range,
                description,
                replacement_text,
                label: label.to_string(),
                ty: ty.to_string(),
                counter: *counter,
            };

            rewrites.push(figure);
            *counter += 1;
        }

        let mut output = String::new();
        let mut last_copied = 0;
        for figure in rewrites {
            let counter = figure.counter + 1;
            output.push_str(&content[last_copied..figure.input_range.start]);
            last_copied = figure.input_range.end;

            let name = format!("{ty} {counter}", ty = figure.ty);
            let description = if let Some(description) = figure.description {
                format!("{name}: {description}")
            } else {
                name.clone()
            };

            #[rustfmt::skip]
            writeln!(
                output,
r#"<div class="figure" id="{label}">

{replacement_text}

<div class="figure-caption">

{description}

</div>
</div>

[](label:{label} "{name}")
"#,
                label = figure.label,
                replacement_text = figure.replacement_text,
            )
            .context("writing output")?;
        }

        output.push_str(&content[last_copied..]);

        chapter.content = output;

        number_figures(&mut chapter.sub_items, counters)?;
    }

    Ok(())
}
