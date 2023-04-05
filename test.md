@{
    ViewBag.Title = "Test";
    Model.Y = Model.X == "`what`";
}

# This is *the* Title

Here is some *text*. It is in a paragraph with two sentences, the second one has a [link](https://google.com) to google.

## Here is an image of a cat.

![Cat image](https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/1200px-Cat03.jpg)

### Heading 3

This is the end of header-testing, so there is a line separator here.

---

Double star looks like **this** and has a space after it.

Triple star looks like ***this***.

Underscore looks like _this_.

Double underscore looke like __this__.

With wigglies it looks like ~~this~~.

> Everything you read on the internet is true. 
>
> *- Barack Obama*

A list:
 - First thing
 - Second thing
 - Last item

Instructions:
 1. Make an ordered list
 2. Make sure it's numbered
 3. Follow it in order

Completion:
 - [x] Start a checklist
 - [ ] Check everything off

List with items far apart:
 - Hi down there!


 - Hello  from down here!

```{warning}
The code below will panic!
```

```
fn main() {
    panic!("Hello World")
}
```

```rust
fn main() {
    panic!("Hello World")
}
```

A way to make it not panic would be to use `println!()` instead of `panic!()`.


| Tables |  Are  | Cool  |
|--------|-------|-------|
| col 1  | col 2 | col 3 |
| col 4  | col 5 | col 6 |

Here is an indented block:

    hello, im an indented block ☺️
        and i can be indented even more!

This is very strange. [Red Link]

TODO: html tag elements
TODO: wiki-links

```sh
ffmpeg -i "my input video.mp4" -vf scale=1920:1080 -sws_flags neighbor "my output video.mp4"
```

::: info
this area is fenced with `::: info`
:::