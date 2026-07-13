use anyhow::Result;
use mdbook_preprocessor::book::{Book, BookItem, Chapter};
use pulldown_cmark::{CowStr, Event, LinkType, Parser, Tag, TagEnd};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
};

mod rewrite;

use crate::rewrite::{Rewrite, Rewrites};

#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
struct Crossref {
    path: PathBuf,
    anchor: String,
    supplement: Option<String>,
}

impl Crossref {
    pub fn rel_link_from(&self, from: &Path) -> String {
        let mut result = PathBuf::new();
        let parent_components = from.parent().unwrap().components();

        for _ in parent_components {
            result.push("..");
        }

        result.push(&self.path);

        format!(
            "{path}#{anchor}",
            path = result.display(),
            anchor = self.anchor
        )
    }
}

type LinkMap<'a> = HashMap<PathBuf, Vec<Link<'a>>>;

#[derive(Default)]
pub struct CrossrefPreprocessor;

impl CrossrefPreprocessor {
    pub fn rewrite_book(book: &mut Book) -> Result<()> {
        let mut map = Default::default();
        let mut rewrites = Default::default();

        extract_links(&book.items, &mut map);

        let crossrefs = rewrite_and_scan_labels(&mut rewrites, &mut map)?;
        rewrite_refs(&mut rewrites, &map, &crossrefs)?;

        rewrites.apply(&mut book.items);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Url<'a>(CowStr<'a>);

impl<'a> Url<'a> {
    pub fn new(value: CowStr<'a>) -> Option<Self> {
        value.contains(':').then_some(Self(value))
    }

    pub fn protocol(&self) -> &str {
        self.0.split_once(':').unwrap().0
    }

    pub fn value(&self) -> &str {
        self.0.split_once(':').unwrap().1
    }
}

#[derive(Debug, Clone)]
pub struct Link<'a> {
    pub url: Url<'a>,
    pub full_range: Range<usize>,
    pub title: CowStr<'a>,
    pub text: Option<&'a str>,
}

impl<'a> Link<'a> {
    pub fn new(
        url: Url<'a>,
        full_range: Range<usize>,
        title: CowStr<'a>,
        text: Option<&'a str>,
    ) -> Self {
        Self {
            url,
            full_range,
            title,
            text,
        }
    }
}

/// Extract elements recursively.
///
/// `map` will contain the paths to included
/// chapthers in summary order.
fn extract_links<'a>(items: &'a Vec<BookItem>, map: &mut LinkMap<'a>) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.clone() else {
            continue;
        };

        let items = extract_links_chapter(&chapter);
        map.insert(path, items);

        extract_links(&chapter.sub_items, map);
    }
}

fn extract_links_chapter(chapter: &Chapter) -> Vec<Link<'_>> {
    let mut elements = Vec::new();
    let content = &chapter.content;
    let mut parser = Parser::new(&content).into_offset_iter();

    while let Some((event, range)) = parser.next() {
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                let Some(dest_url) = Url::new(dest_url) else {
                    continue;
                };

                let text = if link_type != LinkType::Autolink {
                    let (mut current, mut current_range) = parser.next().unwrap();
                    let start = current_range.start;
                    let mut end = current_range.end;
                    loop {
                        if current == Event::End(TagEnd::Link) {
                            break;
                        } else {
                            end = current_range.end;
                        }

                        let (next, range) = parser.next().unwrap();
                        current = next;
                        current_range = range;
                    }

                    Some(&content[start..end])
                } else {
                    None
                };

                elements.push(Link::new(dest_url, range, title, text));
            }
            _ => {}
        }
    }

    elements
}

fn rewrite_and_scan_labels(
    rewrites: &mut Rewrites,
    map: &LinkMap,
) -> Result<HashMap<String, Crossref>> {
    let mut known_crossrefs = HashMap::new();

    for (md_path, links) in map {
        let rewrites_path = rewrites.at(md_path.clone());
        for link in links {
            if link.url.protocol() != "label" {
                continue;
            }

            let id = link.url.value();

            let supplement = if !link.title.is_empty() {
                Some(link.title.to_string())
            } else {
                None
            };

            let existing = known_crossrefs.insert(
                id.to_string(),
                Crossref {
                    path: md_path.clone(),
                    anchor: id.to_string(),
                    supplement,
                },
            );

            if existing.is_some() {
                anyhow::bail!("Duplicate label '{id}'");
            }

            // Render in-place
            let replacement = if let Some(text) = link.text {
                let mut replacement = format!(r#"<span id="{id}">"#);
                replacement.push_str(text);
                replacement.push_str("</span>");
                replacement
            } else {
                "".to_string()
            };

            rewrites_path.push(Rewrite {
                range: link.full_range.clone(),
                replacement,
            });
        }
    }

    Ok(known_crossrefs)
}

fn rewrite_refs(
    rewrites: &mut Rewrites,
    map: &LinkMap,
    crossrefs: &HashMap<String, Crossref>,
) -> Result<()> {
    // Rewrite all links
    for (md_path, links) in map {
        let rewrites = rewrites.at(md_path.clone());
        for link in links {
            if link.url.protocol() != "ref" {
                continue;
            }

            let Some(crossref) = crossrefs.get(link.url.value()) else {
                anyhow::bail!("Unknown reference `{}`", link.url.value());
            };

            let supplement = if let Some(text) = link.text {
                text
            } else if let Some(supp) = &crossref.supplement {
                supp.as_ref()
            } else {
                eprintln!(
                    "Cross-reference `{}` has neither supplement nor text",
                    link.url.0
                );
                link.url.0.as_ref()
            };

            let title = link.title.replace(r#"""#, r#"\""#);
            let replacement = format!(
                r#"[{supplement}]({url} "{title}")"#,
                url = crossref.rel_link_from(md_path)
            );

            let rewrite = Rewrite {
                range: link.full_range.clone(),
                replacement,
            };

            rewrites.push(rewrite);
        }
    }

    Ok(())
}
