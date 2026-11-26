# Third-party notices

## Rust dependencies

This section is generated for each release by
`cargo about generate about.hbs > THIRD_PARTY_NOTICES.generated.md` and
appended here by the release workflow.

## Fonts (website)

The website under `site/` serves Martian Mono (Copyright 2022 The Martian Mono Project Authors, https://github.com/evilmartians/mono; the variable font with width and weight axes) and Archivo (Copyright 2020 The Archivo Project Authors, https://github.com/Omnibus-Type/Archivo; the variable font with the weight axis) as WOFF2 files, Latin subsets as Google Fonts distributes them, both under the SIL Open Font License 1.1 (https://openfontlicense.org).

## three.js (website)

The website under `site/` serves three.js r170 (`three@0.170.0`, the file
`build/three.module.min.js` of the npm package, unchanged, as
`site/vendor/three.module.min.js`; https://threejs.org) for the paper crane in
the hero. Its licence is copied next to it as `site/vendor/three.LICENSE`.

The MIT License

Copyright © 2010-2024 three.js authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

## References consulted

origami is an independent implementation. These projects were consulted as
references for statement coverage, test cases and layout conventions:

| Project | Version consulted | License |
|---|---|---|
| abaplint (`@abaplint/core`) | 2.120.112 | MIT |
| ABAP Cleaner | 1.29.2 | Apache-2.0 |

No source code was copied. Test inputs taken from abaplint are listed below
with their license. (None so far.)

## Test corpus

The corpus in `corpus/manifest.toml` is fetched at test time and never
redistributed. Each entry records its SPDX license.

## abaplint (test data only, not shipped)

`corpus/catalog/abaplint-*.json` contains example statements extracted from
the abaplint test suite (https://github.com/abaplint/abaplint).

MIT License. Copyright (c) 2015 Lars Hvam.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
