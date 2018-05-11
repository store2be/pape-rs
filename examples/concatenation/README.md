# PDF concatenation

This example can be run with the `papers local` test command.

The Latex template uses the `pdfpages` package to include other pdfs. This way,
papers can be used as a concatenation tool.

**Note:** The links to the PDFs need to be both in `assets_urls` and
`variables.concat_pdfs` with this setup.

## Example request

```json
{
    "assets_urls": [
        "https://example.com/papers_logo.png",
        "https://example.com/woosh.jpeg",
        "https://example.com/vektoranalysis_teil_1.pdf",
        "https://example.com/linux_praxisbuch_shellprogrammierung",
        "https://example.com/batch_programmierung.pdf"
    ],
    "template_url": "https://example.com/templates/merge.tex.tera",
    "callback_url": "https://example.com/callback&token=914f07aa876128e936",
    "variables": {
        "concat_pdfs": [
            "papers_logo.png",
            "woosh.jpeg",
            "vektoranalysis_teil_1.pdf",
            "linux_praxisbuch_shellprogrammierung",
            "batch_programmierung.pdf"
        ],
    }
}
```

## or just run the local server to try it out

(See [simple example](../simple))

```
$ cd examples/concatenation
$ papers local
```
