
## Feature Comparison

Due to missing/unclear documentation and scope limitations, this is only a partial comparison.

*That said, if you find this information to be inaccurate, please let me know.*

| Feature \ Tool            | [kumono][kmn]      | [KToolBox][ktb]                | [gallery-dl][gdl] | [Kemono-and-Coomer-Downloader][kacd] |
| ------------------------- | ------------------ | ------------------------------ | ----------------- | ------------------------------------ |
| language                  | Rust               | [Python][ktb-py]               | [Python][gdl-py]  | [Python][kacd-py]                    |
| issues (besides Python)   | beta software      | lack of features and UX        | no concurrency    | no concurrency                       |
| concurrency               | yes (default 256)  | yes (default 10)               | no                | no                                   |
| parse multiple URLs       | yes                | ?                              | yes               | yes                                  |
| kemono support            | yes                | yes                            | yes               | yes                                  |
| coomer support            | yes                | [extra config required][ktb-c] | yes               | yes                                  |
| user interface            | CLI                | CLI                            | CLI               | TUI (kind of a chore to use)         |
| direct URL parser         | yes                | yes                            | yes               | no                                   |
| creator all posts         | yes                | yes                            | yes               | yes                                  |
| creator single page       | yes                | manual                         | yes               | yes                                  |
| creator single post       | yes                | yes                            | yes               | yes                                  |
| creator + linked accounts | yes                | no                             | no                | no                                   |
| discord server            | yes                | no                             | yes               | ?                                    |
| discord channel           | yes                | no                             | no                | ?                                    |
| favorites (creator)       | [planned][favs]    | ?                              | ?                 | ?                                    |
| favorites (post)          | [planned][favs]    | ?                              | ?                 | ?                                    |
| DM/fancard/community      | [planned][dms-etc] | ?                              | ?                 | ?                                    |
| verify hashes             | yes                | ?                              | ?                 | ?                                    |
| resume downloads          | yes                | ?                              | ?                 | ?                                    |
| skip existing download    | yes                | yes                            | yes               | ?                                    |
| download archive          | yes                | ?                              | ?                 | ?                                    |
| file type filtering       | yes                | yes                            | yes               | ?                                    |
| proxy support             | yes                | ?                              | yes               | ?                                    |
| retry on timeout          | planned            | ?                              | ?                 | ?                                    |
| retry on server error     | yes                | yes                            | ?                 | ?                                    |
| retry on connection error | yes                | ?                              | yes               | ?                                    |

<!-- | use original file name    | no (TBD?)         | ?                              | ?                 | ?                                    | -->
<!-- | advanced renaming options | no (TBD?)         | yes                            | ?                 | ?                                    | -->

*For GUI options, see [VoxDroid/KemonoDownloader][vdkd] and [Yuvi9587/Kemono-Downloader][yuvikd]. I haven't found the time to compare them to `kumono` yet.*

<!-- link definitions -->

[kmn]: https://github.com/APT37/kumono

[ktb]: https://github.com/Ljzd-PRO/KToolBox
[ktb-py]: https://github.com/Ljzd-PRO/KToolBox/issues?q=is%3Aissue%20python
[ktb-c]: https://ktoolbox.readthedocs.io/latest/coomer/

[gdl]: https://github.com/mikf/gallery-dl
[gdl-py]: https://github.com/mikf/gallery-dl/issues?q=is%3Aissue%20python

[kacd]: https://github.com/e43b/Kemono-and-Coomer-Downloader
[kacd-py]: https://github.com/e43b/Kemono-and-Coomer-Downloader/issues?q=is%3Aissue%20python

[favs]: https://github.com/APT37/kumono/issues/5
[dms-etc]: https://github.com/APT37/kumono/issues/3

[vdkd]: https://github.com/VoxDroid/KemonoDownloader
[yuvikd]: https://github.com/Yuvi9587/Kemono-Downloader
