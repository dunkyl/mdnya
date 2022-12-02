# MDNya

Convert one markdown file to one html file.

## Usage

mdnya is a command line tool. You can use it like this:
```sh
mdnya README.md
```
It would produce a file named `README.html` in the same directory.

TODO: libary usage

## Quirks you might like

 - By default, `<p>` and `<li>` tags are not closed
 - li elements do not contain a `<p>` tag
 - indented code blocks are treaded instead as indented text
 - An option to wrap the elements between headers in a `<section>` or other tag
 - Headers get their content added as an id attribute, so you can link to them
    - With some care to respect Razor syntax, for anything such as `@ViewData["Title"]`
 - TODO: parse with Razor to preserve all Razor syntax, such as @{ } blocks and @model directives
 - Language highlighters can be loaded dynamically as plugins
 - TODO: hashtags are formatted and can be scraped from the document
 - highlight configuration is pre-calculated at compile time for both statically linked and dynamically loaded syntax highlighters, which can speed up processing time for small documents considerably

## Extensions

Git markdown extensions are supported, such as:
 - checkboxes in lists
 - tables

In addition to the following:
 - Admonitions for code blocks

The syntax for admonitions is:
````md
```{kind} An optional custom title
    The text that shows inside!
```
````
Where `kind` can be any class. The HTML div for this admonition will have the classes `admonition kind`.

---

## Building

Tree Sitter Language(s):
 - Markdown (REQUIRED) - [tree-sitter-markdown](https://github.com/ikatyang/tree-sitter-markdown) by ikatyang
 If building mdnya-cli, by default it statically links the following languages for syntax highlighting:
    - C#
    - Rust
    - Bash
    TODO: add more languages

Other parsers will be used inside code blocks for highlighting. Clone each parser repo into the `mdnya-hl-langs/tree-sitters` directory.

TODO: about static and dynamic linking of highlighters

---

## More on usage

By default, the outputted HTML code will not close optional tags. This is to make the outputted HTML code more readable. If you want to close optional tags, use the `--close-optional-tags` flag.

The output can be written to a specific file by using the `--output` flag, or to stdout by using the `--output stdout`.

The elements surrounding the markdown content can be customized by using the `--wrap-tags` flag. The default value is none, and the elements are at the top level. The `--wrap-tags` flag can be passed multiple, comma separated values for nested elements. For example, `--wrap-tags div,article` will wrap the markdown content in a div, and then wrap the div in an article.