# MDNya

Convert one markdown file to one html file.

## Usage

mdnya is a command line tool. You can use it like this:

    mdnya README.md

It would produce a file named `README.html` in the same directory.

TODO: libary usage

## Extensions

Git markdown extensions are supported. In addition to the following:
 - Admonitions for code blocks

---

## Building

Tree Sitter Language(s):
 - Markdown (REQUIRED) - [tree-sitter-markdown](https://github.com/ikatyang/tree-sitter-markdown) by ikatyang

Other parsers will be used inside code blocks for highlighting. Clone each parser repo into `langs` directory.

TODO: about static and dynamic linking of highlighters

---

## More on usage

By default, the outputted HTML code will not close optional tags. This is to make the outputted HTML code more readable. If you want to close optional tags, use the `--close-optional-tags` flag.

The output can be written to a specific file by using the `--output` flag, or to stdout by using the `--output stdout`.

The elements surrounding the markdown content can be customized by using the `--wrap-tags` flag. The default value is none, and the elements are at the top level. The `--wrap-tags` flag can be passed multiple, comma separated values for nested elements. For example, `--wrap-tags div,article` will wrap the markdown content in a div, and then wrap the div in an article.