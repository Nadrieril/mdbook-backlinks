use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use clap::{App, Arg, SubCommand};
use itertools::Itertools;
use mdbook_markdown::pulldown_cmark::{CowStr, Event, HeadingLevel, LinkType, Tag};
use pathdiff;
use semver::{Version, VersionReq};

use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};

/// Helper to build a pulldown_cmark document.
#[derive(Default)]
struct MarkdownBuilder<'a>(Vec<Event<'a>>);

impl<'a> MarkdownBuilder<'a> {
    pub fn write_to_string(self, s: &mut String) {
        let _ = pulldown_cmark_to_cmark::cmark(self.0.into_iter(), s);
    }

    /// Use when none of the other helpers suffice.
    pub fn event(&mut self, event: Event<'a>) {
        self.0.push(event);
    }

    /// Emit a piece of text.
    pub fn text(&mut self, txt: impl Into<CowStr<'a>>) {
        self.event(Event::Text(txt.into()));
    }

    /// Start a tag before the input closure and end it afterwards.
    pub fn tag(&mut self, tag: Tag<'a>, f: impl FnOnce(&mut Self)) {
        let end = tag.to_end();
        self.event(Event::Start(tag));
        f(self);
        self.event(Event::End(end));
    }

    pub fn simple_heading(&mut self, level: HeadingLevel, f: impl FnOnce(&mut Self)) {
        self.tag(
            Tag::Heading {
                level,
                id: None,
                classes: vec![],
                attrs: vec![],
            },
            f,
        )
    }
    pub fn simple_link(&mut self, dest_url: impl Into<CowStr<'a>>, f: impl FnOnce(&mut Self)) {
        self.tag(
            Tag::Link {
                link_type: LinkType::Inline,
                dest_url: dest_url.into(),
                title: "".into(),
                id: "".into(),
            },
            f,
        )
    }
}

fn process_book(mut book: Book) -> Result<Book, Error> {
    // Map each chapters source_path to its backlinks.
    let mut backlinks_map: HashMap<PathBuf, Vec<_>> = HashMap::new();

    // Add entries for the book chapters (so that we don't accumulate links that point outside
    // the book).
    for item in book.iter() {
        if let BookItem::Chapter(ch) = item
            && let Some(path) = &ch.source_path
        {
            backlinks_map.insert(path.clone(), Vec::new());
        }
    }

    // Populate the map.
    for item in book.iter() {
        if let BookItem::Chapter(ch) = item
            && let Some(path) = &ch.source_path
        {
            // Loop over the internal links found in the chapter
            for event in mdbook_markdown::new_cmark_parser(&ch.content, &Default::default()) {
                if let Event::Start(Tag::Link { dest_url, .. }) = event {
                    let dest_chapter = PathBuf::from(&*dest_url);
                    if let Some(backlinks) = backlinks_map.get_mut(&dest_chapter) {
                        let name = ch.name.clone();
                        let path = path.clone();
                        backlinks.push((name, path));
                    }
                }
            }
        }
    }

    // Add backlinks to each chapter.
    for item in &mut book.sections {
        if let BookItem::Chapter(ch) = item
            && let Some(source_path) = &ch.source_path
            && let Some(backlinks) = backlinks_map.get(source_path)
            && backlinks.len() >= 1
        {
            ch.content += "\n\n"; // Avoid the ruler being parsed as a heading underline
            let mut builder = MarkdownBuilder::default();
            builder.event(Event::Rule);
            builder.tag(Tag::BlockQuote(None), |builder| {
                builder.simple_heading(HeadingLevel::H4, |builder| {
                    builder.text("Backlinks");
                });
                builder.tag(Tag::List(None), |builder| {
                    for (name, path) in backlinks.iter().sorted().dedup() {
                        let diff_path =
                            pathdiff::diff_paths(path, source_path.parent().unwrap()).unwrap();
                        let dest_url = diff_path.to_str().unwrap().to_owned();
                        builder.tag(Tag::Item, |builder| {
                            builder.simple_link(dest_url, |builder| {
                                builder.text(name.as_str());
                            });
                        });
                    }
                });
            });
            builder.write_to_string(&mut ch.content);
        }
    }

    Ok(book)
}

pub fn make_app() -> App<'static, 'static> {
    App::new("mdbook-backlinks")
        .about("A mdbook preprocessor which inserts backlinks")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

pub struct Backlinks;
impl Preprocessor for Backlinks {
    fn name(&self) -> &str {
        "backlinks"
    }

    fn run(&self, _ctx: &PreprocessorContext, book: Book) -> Result<Book, Error> {
        process_book(book)
    }
}

fn main() -> Result<(), Error> {
    let matches = make_app().get_matches();
    if let Some(_) = matches.subcommand_matches("supports") {
        // We support all renderers
    } else {
        handle_preprocessing(&Backlinks)?;
    }
    Ok(())
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if version_req.matches(&book_version) != true {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}
