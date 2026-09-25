# Fonts

Three families, all under the SIL Open Font License 1.1, which allows use on a
website, bundling the files and serving them from our own domain, as long as the
licence text ships with them. It sits next to the fonts here.

| files | family | version | taken from |
| --- | --- | --- | --- |
| `writer-*.woff2` | iA Writer Quattro S | repo master | [iaolo/iA-Fonts](https://github.com/iaolo/iA-Fonts), `iA Writer Quattro/Webfonts` |
| `plexserif-var*.woff2` | IBM Plex Serif Var | 1.000 | npm [`@ibm/plex-serif-variable`](https://www.npmjs.com/package/@ibm/plex-serif-variable) 2.0.0 |
| `lilex-regular.woff2` | Lilex | 2.621 | [mishamyrt/Lilex](https://github.com/mishamyrt/Lilex/releases/tag/2.621) release zip |

Every file here was checked against the upstream release and is byte for byte
the vendor's own, not a subset or a re-export. That matters for the licence: an
unmodified font can keep its reserved name, a modified one cannot. So if any of
these needs replacing, take the new file from the source above rather than
subsetting it, and keep the matching licence file.
