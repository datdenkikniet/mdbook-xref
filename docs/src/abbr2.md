# The `mdbook-abbr2` preprocessor

This preprocessor, in combination with the `mdbook-xref` preprocessor, provides simple abbreviation
support for your mdBook.

The abbreviations will be automatically expanded like `Comma Separated Value (CSV)` for the first encounter in each
chapter. To disable this auto-expansion, set the `preprocessor.abbr2.auto-expand` configuration key to `false` in your `book.toml`.

Abbreviations are defined in a <abbr:CSV> file (whose path is configured using the `preprocessor.abbr2.path` configuration key
in your `book.toml`) in the following format:

```
<Abbreviation>, <Description>[, <Optional hover text>]
# For example
CSV, Comma Separated Value
# And with custom hover text
CAB, Complicated Abbreviation with a very long description that's unsuitable for hover text, Complicated Abbreviation
```

Referring to abbreviations can then be done by using autolinks with the `abbr/noabbr` scheme:

```
When writing a <abbr:CSV> file, make sure to escape double quotes with double quotes, and
<abbr:CAB> to explain other concepts. If you want to, for some reason, not mark it as an
abbreviation, writing <noabbr:CSV> will work fine.
```

renders as:

When writing a <abbr:CSV> file, make sure to escape double quotes with double quotes, and
<abbr:CAB> to explain other concepts. If you want to, for some reason, not mark it as an
abbreviation, writing <noabbr:CSV> will work fine.

Abbreviations expand to links in the abbreviations page, which is appended to the end of the book, with a separator.
To disable the chapter separator, set the `preprocessor.abbr2.separator` configuration key to `false`.

When referencing an abbreviation for the first time, it will be expanded to it's full meaning. All consecutive references
to that abbreviation in a chapter will only display in short form. Example: 
> <abbr:HTML> is a markup language for creating webpage structure. <abbr:HTML> can either be written pure, or with a framework.

## Auto-checks

By default, the `preprocessor.abbr2.validate` configuration key is set to `warn`. To turn off validation,
set the `preprocessor.abbr2.validate` configuration key to `quiet`. To enable the preprocessor producing
errors, set the `preprocessor.abbr2.validate` configuration key to `error`.

All non-marked words will be cross-referenced against the active lists of abbreviations. If any word is found in the list that isn't marked
the preprocessor will produce an error, and try its best to explain where and why it failed.

The check will not consider text within code blocks.

```
CSV will be flagged as not marked, while <noabbr:CSV> and <abbr:CSV> are considered valid.
```

## Getting started

To get started, install the preprocessors:

```sh
cargo install mdbook-xref mdbook-abbr2
```

and add the required configuration to your `book.toml`:

```
[preprocessor.xref]

[preprocessor.abbr2]
before = ["xref"]
path = "abbreviations.csv"
```
