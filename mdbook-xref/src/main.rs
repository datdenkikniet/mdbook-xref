use std::io::{Read, Write};

use anyhow::{Context, Result};
use mdbook_preprocessor::book::{Book, BookItem, Chapter};

use mdbook_xref::CrossrefPreprocessor;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let command = args.first();

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
        Some("single-chapter") => single_chapter()?,
        Some(_) => return Err(anyhow::anyhow!("Unknown command")),
        _ => book()?,
    };

    std::io::stdout().write_all(output.as_bytes())?;

    Ok(())
}

fn single_chapter() -> Result<String> {
    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;

    let chapter = Chapter {
        name: "inline".into(),
        content,
        number: None,
        sub_items: Vec::new(),
        path: Some("path".into()),
        source_path: None,
        parent_names: Vec::new(),
    };

    let items = vec![BookItem::Chapter(chapter)];

    let mut book = Book { items };

    CrossrefPreprocessor::rewrite_book(&mut book)?;

    let BookItem::Chapter(output) = book.items.remove(0) else {
        panic!("Time to remove single-chapter");
    };

    Ok(output.content)
}

fn book() -> Result<String> {
    let (_ctx, mut book) = mdbook_preprocessor::parse_input(std::io::stdin())?;
    CrossrefPreprocessor::rewrite_book(&mut book)?;
    Ok(serde_json::to_string(&book)?)
}
