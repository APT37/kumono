
## Feature Comparison

Due to missing documentation, this is only a partial list.

*That said, if you find this information to be inaccurate, please let me know.*

| Feature \ Tool                | kumono            | [KToolBox][ktb]                      | [gallery-dl][gdl] | [Kemono-and-Coomer-Downloader][kacd] |
| ----------------------------- | ----------------- | ------------------------------------ | ----------------- | ------------------------------------ |
| language                      | Rust              | [Python][ktb-py]                     | [Python][gdl-py]  | [Python][kacd-py]                    |
| issues (besides Python)       | many features TBD | [high complexity][ktb-c], messy docs | no concurrency    | no concurrency                       |
| user interface                | CLI               | CLI                                  | CLI               | TUI (kind of a chore to use)         |
| direct URL parser             | yes               | yes                                  | yes               | no                                   |
| concurrency                   | yes (default 256) | yes (default 10)                     | no                | no                                   |
| creator all posts             | yes               | yes                                  | yes               | yes                                  |
| creator single page           | no (TBD)          | ?                                    | ?                 | yes                                  |
| creator single post           | yes               | yes                                  | yes               | yes                                  |
| creator + all linked accounts | not (TBD)         | no                                   | no                | no                                   |
| discord server                | yes               | no                                   | yes               | ?                                    |
| discord channel               | yes               | no                                   | no                | ?                                    |
| favorites (creator)           | no (TBD)          | ?                                    | ?                 | ?                                    |
| favorites (post)              | no (TBD)          | ?                                    | ?                 | ?                                    |
| skip existing files           | yes               | yes                                  | yes               | ?                                    |
| file type filtering           | yes               | yes                                  | yes               | ?                                    |
| proxy                         | yes               | ?                                    | yes               | ?                                    |
| retry API calls               | no (TBD)          | yes                                  | ?                 | ?                                    |
| retry (server error)          | yes               | yes                                  | ?                 | ?                                    |
| retry (connection error)      | no (TBD)          | ?                                    | yes               | ?                                    |
| resume downloads              | yes               | ?                                    | ?                 | ?                                    |
| verify hashes after download  | yes               | ?                                    | ?                 | ?                                    |
| use original file name        | no (TBD?)         | ?                                    | ?                 | ?                                    |
| download archive              | no (TBD?)         | ?                                    | yes               | ?                                    |
| Advanced renaming options     | no                | yes                                  | ?                 | ?                                    |

<!-- link definitions -->

[ktb]: https://github.com/Ljzd-PRO/KToolBox
[ktb-py]: https://github.com/Ljzd-PRO/KToolBox/issues?q=is%3Aissue%20python
[ktb-c]: https://github.com/Ljzd-PRO/KToolBox/issues/141

[gdl]: https://github.com/mikf/gallery-dl
[gdl-py]: https://github.com/mikf/gallery-dl/issues?q=is%3Aissue%20python

[kacd]: https://github.com/e43b/Kemono-and-Coomer-Downloader
[kacd-py]: https://github.com/e43b/Kemono-and-Coomer-Downloader/issues?q=is%3Aissue%20python