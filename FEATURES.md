
## Feature Comparison

Due to missing/unclear documentation and scope limitations, this is only a partial comparison.

*That said, if you find this information to be inaccurate, please let me know.*

| Feature \ Tool            | [kumono][kmn]     | [KToolBox][ktb]                | [gallery-dl][gdl] | [Kemono-and-Coomer-Downloader][kacd] |
| ------------------------- | ----------------- | ------------------------------ | ----------------- | ------------------------------------ |
| language                  | Rust              | [Python][ktb-py]               | [Python][gdl-py]  | [Python][kacd-py]                    |
| issues (besides Python)   | some features TBD | lack of features and UX        | no concurrency    | no concurrency                       |
| kemono support            | yes               | yes                            | yes               | yes                                  |
| coomer support            | yes               | [extra config required][ktb-c] | yes               | yes                                  |
| user interface            | CLI               | CLI                            | CLI               | TUI (kind of a chore to use)         |
| direct URL parser         | yes               | yes                            | yes               | no                                   |
| read multiple URLs        | yes               | ?                              | yes               | yes                                  |
| concurrency               | yes (default 256) | yes (default 10)               | no                | no                                   |
| creator all posts         | yes               | yes                            | yes               | yes                                  |
| creator single page       | yes               | manual                         | yes               | yes                                  |
| creator single post       | yes               | yes                            | yes               | yes                                  |
| creator + linked accounts | no (TBD)          | no                             | no                | no                                   |
| discord server            | yes               | no                             | yes               | ?                                    |
| discord channel           | yes               | no                             | no                | ?                                    |
| favorites (creator)       | no (TBD)          | ?                              | ?                 | ?                                    |
| favorites (post)          | no (TBD)          | ?                              | ?                 | ?                                    |
| skip existing files       | yes               | yes                            | yes               | ?                                    |
| file type filtering       | yes               | yes                            | yes               | ?                                    |
| proxy support             | yes               | ?                              | yes               | ?                                    |
| retry on server error     | yes               | yes                            | ?                 | ?                                    |
| retry on connection error | yes               | ?                              | yes               | ?                                    |
| resume downloads          | yes               | ?                              | ?                 | ?                                    |
| verify hashes             | yes               | ?                              | ?                 | ?                                    |

<!-- | use original file name    | no (TBD?)         | ?                              | ?                 | ?                                    | -->
<!-- | download archive          | no (TBD?)         | ?                              | yes               | ?                                    | -->
<!-- | advanced renaming options | no (TBD?)         | yes                            | ?                 | ?                                    | -->

<!-- link definitions -->

[kmn]: https://github.com/APT37/kumono

[ktb]: https://github.com/Ljzd-PRO/KToolBox
[ktb-py]: https://github.com/Ljzd-PRO/KToolBox/issues?q=is%3Aissue%20python
[ktb-c]: https://ktoolbox.readthedocs.io/latest/coomer/

[gdl]: https://github.com/mikf/gallery-dl
[gdl-py]: https://github.com/mikf/gallery-dl/issues?q=is%3Aissue%20python

[kacd]: https://github.com/e43b/Kemono-and-Coomer-Downloader
[kacd-py]: https://github.com/e43b/Kemono-and-Coomer-Downloader/issues?q=is%3Aissue%20python
