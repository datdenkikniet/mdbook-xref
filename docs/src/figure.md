# The `mdbook-figure` preprocessor

This preprocessor allows you to define figures with a label, an optional type, a description, and contents.

## Quick Start

To get started, install the preprocessor:

```sh
cargo install mdbook-figure
```

and add it to your `book.toml`:

```toml
[book]

[preprocessor.figure]
# If you're also using `mdbook-xref` and/or `mdbook-abbr2`, you must use the
# correct ordering for the both of them to work as expected:
before = [ "xref", "abbr2" ]
```

## Defining figures

Defining figures is done as follows:

````
```figure a-label Optional Type
The first line is the description of the figure, which **can** _contain_ `markdown`, and is a hyperlink to the figure. The description can also contain <abbr:ABBR>s that expand correctly.
<center>
The rest describes its contents, which are rendered as

**markdown** _in_ `the` final document.
</center>
```
````

which renders as

```figure a-label Optional Type
The first line is the description of the figure, which **can** _contain_ `markdown`, and is a hyperlink to the figure. The description can also contain <abbr:ABBR>s that expand correctly.
<center>
The rest describes its contents, which are rendered as

**markdown** _in_ `the` final document.
</center>
```

The figures are numbered by type and order in the book.

These figures can be referred to by their label using the `mdbook-xref` preprocessor. In this case, we can refer to [`ref:a-label`](ref:a-label), or <ref:a-label>.

The caption itself also links to its figure, so that auto-navigatable links can be created easily without requiring other in-text references.

## Autodetection

If the type of the figure is not specified, it defaults to "Figure". However, the type is automatically inferred for some content, so long as that
content immediately follows the description.

Currently, only "Table" is supported:

````
```figure a-table
a very fancy table
| Column 1 | Column 2 |
| :------- | :------- |
| Value1   | Value2   |
```
````

which renders as

```figure a-table
a very fancy table
| Column 1 | Column 2 |
| :------- | :------- |
| Value1   | Value2   |
```

and is referred to as <ref:a-table>

## Styling

With the HTML renderer, figures are turned into `div` elements with the `figure` class. Additionally, the figure caption is inserted as a `div` element with the `figure-caption` class.

The clickable captions generally inherit `<a>` styling from the browser, which can look odd. Restyling those links is relatively straightforward:

```css
.figure-caption a {
    color: inherit !important;
}
```
