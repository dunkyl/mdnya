# mdnya

Convert one markdown file to one html file.

## Usage

mdnya is a command line tool. You can use it like this:
```sh
mdnya README.md --meta --output basic.md.html
```
It would produce a file named `basic.md.html` and one named `README.json` in the same directory.

```jsonc
{ // README.json as generated for mdnya's readme
  "title": "mdnya",
  "tags": [
    "hashtags"
  ],
  "frontmatter": {}
}
```

## Requirements

- Nodejs 14 or higher

## Quirks you might like

- By default, `<p>` and `<li>` tags are not closed
- li elements do not contain a `<p>` tag
- standalone images are not wrapped in a `<p>` tag
- An option to wrap the elements between headers in a `<section>` or other tag
- Headers get their content can added as an id attribute, so you can link to them
- Fenced (```) code blocks with an @ are preserved as razor @{ } blocks
- Hashtags are formatted and collected from the document. #hashtags are added to the `tags` field in the metadata file.
- Frontmatter is parsed as YAML and added to the `frontmatter` field in the metadata file.

## Extensions

Git markdown extensions are supported, such as:
- Checkbox lists
- Tables, including alignment

In addition to the following:
- Tables can be captioned:
```md
| Example | Table |
|:--------|:------|
| 1       | 2     |
| 3       | 4     |

: The above table is an example table.
```

- Admonitions:
````md
```{kind} An optional custom title
The text that shows inside!
```
````
Where `kind` can be any class. The HTML div for this admonition will have the classes `admonition kind`.
```html
<div class="admonition kind">
    <h3>An optional custom title</h3>
    <p>The text that shows inside!
</div>
```

---

## Building

Highlighting is done with [Starry Night](https://github.com/wooorm/starry-night), which is a javascript library. It is bundled with webpack. Both building and running mdnya requires nodejs.
```sh
npm i
webpack
```

---

## More on usage

By default, the outputted HTML code will not close optional tags. This is to make the outputted HTML code more readable. If you want to close optional tags, use the `--close-optional-tags` flag.

The output can be written to a specific file by using the `--output` flag, or to stdout by using the `--output stdout`.

The elements surrounding the markdown content can be customized by using the `--doc-tags` flag. The default value is none, and the elements are at the top level. The `--doc-tags` flag can be passed multiple, comma separated values for nested elements. For example, `--doc-tags div,article` will wrap the markdown content in a div, and then wrap the div in an article.
