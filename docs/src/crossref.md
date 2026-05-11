# The `mdbook-xref` preprocessor

This preprocessor allow you to create cross-referenceable IDs by creating new link items
that use the `label` protocol. Additionally, it lets you set the supplement (the text substituted
at the place of reference) for these links, which is used unless the reference specifies its text
explicitly.

The created labels can then be referred to using links with the `ref` protocol.

The benefits of this approach over the usual linking approach using fixed anchors and relative links is
that labels are moveable, as a reference to a label is valid as long as it exists, regardless its location
in the book. Additionally, broken links/references become apparent much more quickly: the preprocessor will
produce an error if the label specified in a `ref` is undefined.

## Quick Start

To get started, install the preprocessor:

```sh
cargo install mdbook-xref
```

and add it to your `book.toml`:

```toml
[book]

[preprocessor.xref] # This is all you need to do
```

and start creating `label`s and `ref`erences.

That's all there is to it! To find out how to actually use the preprocessor in more
detail, see <ref:creating_links>.

If you have other mdbook preprocessors that might produce `label`s or `ref`s, make sure that they
are called _before_ this preprocessor by adding [`before = [ "xref" ]`][config]
to its configuration.

[config]: https://rust-lang.github.io/mdBook/format/configuration/preprocessors.html#require-a-certain-order

## Creating cross-referenceable labels { #creating_links }
[](label:creating_links "creating links")

Any link (inline, reference, autolink) with the `label` protocol will be interpreted as a cross-referenceable ID on the
page where that label is created. That the essence of this preprocessor: it associate an ID with a specific page and lets
you easily reference that ID in other places.

For instance, the preprocessor lets you create a piece of referenceable text (with support for inline markdown) as follows:

```
[a piece of referenceable text](label:a_piece_of_text "an optional supplement")

[a piece of **referenceable** _text_ without supplement, but nice markdown](label:text_without_supplement)
```

which is rendered like this:

[a piece of referenceable text](label:a_piece_of_text "an optional supplement")

[a piece of **referenceable** _text_ without supplement, but nice markdown](label:text_without_supplement)

## Cross-referencing other elements

Non-text elements can also be cross-referenced as follows:

```
##### A referenceable heading { #heading }
[](label:heading "the heading's supplement")
```

Labels without any text are not rendered, but cross-references to `heading`
will create a link to the element with ID of the label, which in this case is `heading`
(using `mdBooks` built-in heading ID support).

It is rendered like this (note that the labelled data is not explicitly visible):

##### A referenceable heading { #heading }
[](label:heading "the heading's supplement")

In general, you should not create cross-referenceable items like this manually. Instead, you
should let other preprocessors generate them for you.

## Referring to cross-referefences { #creating }
[](label:creating "the section on creating references")

A cross reference can be referred to by any links with the `ref` protocol followed by a
label defined in some `label` link.

### Normal inline links

Normal inline links work as expected

```
[A reference to the text.](ref:text_without_supplement)
```

renders as

[A reference to the text.](ref:text_without_supplement)

### Reference links

Reference links are supported:

```
[A reference to the text][1]

[1]: ref:a_piece_of_text
```

renders as

[A reference to the text][1]

[1]: ref:a_piece_of_text

### Autolinks

Autolinks are also supported, but only work if the reference has a supplement:

```
<ref:a_piece_of_text>

<ref:my-heading>
```

renders as

<ref:a_piece_of_text>

<ref:heading>

